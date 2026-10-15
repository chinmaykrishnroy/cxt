//! Lexical binding decisions shared by graph retrieval and local span retention.
//! This module does not read files or claim runtime/type resolution.
use crate::symbols::{ReferenceSite, SourceIndex, SourceSpan, Symbol};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) enum Binding<'a> {
    Local(&'a Symbol),
    Import { index: usize, exported: &'a str },
    Unbound,
    Unresolved,
}

pub(crate) struct ScopeIndex<'a> {
    source: &'a SourceIndex,
    definitions: BTreeMap<&'a str, Vec<&'a Symbol>>,
    class_scopes: BTreeSet<(usize, usize)>,
}

fn contains(scope: &SourceSpan, site: &SourceSpan) -> bool {
    scope.start_byte <= site.start_byte && scope.end_byte >= site.end_byte
}

fn width(scope: &SourceSpan) -> usize {
    scope.end_byte - scope.start_byte
}

impl<'a> ScopeIndex<'a> {
    pub(crate) fn new(source: &'a SourceIndex) -> Self {
        let mut definitions = BTreeMap::<_, Vec<_>>::new();
        for symbol in &source.symbols {
            definitions
                .entry(symbol.qualified_name.as_str())
                .or_default()
                .push(symbol);
        }
        Self {
            source,
            definitions,
            class_scopes: source
                .symbols
                .iter()
                .filter(|s| s.kind == "class_definition")
                .map(|s| (s.span.start_byte, s.span.end_byte))
                .collect(),
        }
    }

    pub(crate) fn resolve<'s>(&'s self, owner: &Symbol, site: &'s ReferenceSite) -> Binding<'s> {
        let name = site.qualifier.as_ref().unwrap_or(&site.name);
        let shadow = self
            .source
            .local_bindings
            .iter()
            .filter(|b| b.name == *name && contains(&b.scope, &site.span))
            .map(|b| width(&b.scope))
            .min();
        let imports: Vec<_> = self
            .source
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                b.local == *name
                    && contains(&b.scope, &site.span)
                    && !(owner.kind == "function_definition"
                        && self
                            .class_scopes
                            .contains(&(b.scope.start_byte, b.scope.end_byte)))
            })
            .collect();
        let import_width = imports.iter().map(|(_, b)| width(&b.scope)).min();
        let imports: Vec<_> = imports
            .into_iter()
            .filter(|(_, b)| Some(width(&b.scope)) == import_width)
            .collect();
        let mut local = Vec::new();
        if site.qualifier.is_none() {
            let mut prefix = owner.qualified_name.as_str();
            loop {
                let qualified = if prefix.is_empty() {
                    site.name.clone()
                } else {
                    format!("{prefix}::{}", site.name)
                };
                let python_class = self
                    .definitions
                    .get(prefix)
                    .is_some_and(|symbols| symbols.iter().any(|s| s.kind == "class_definition"));
                if let Some(found) = self
                    .definitions
                    .get(qualified.as_str())
                    .filter(|_| !python_class)
                {
                    local.extend(
                        found
                            .iter()
                            .copied()
                            .filter(|s| contains(&s.binding_scope, &site.span)),
                    );
                    if !local.is_empty() {
                        break;
                    }
                }
                if prefix.is_empty() {
                    break;
                }
                prefix = prefix.rsplit_once("::").map_or("", |(parent, _)| parent);
            }
        }
        let local_width = local.iter().map(|s| width(&s.binding_scope)).min();
        let nearest = local_width.into_iter().chain(import_width).min();
        if shadow.is_some_and(|s| s <= nearest.unwrap_or(usize::MAX)) {
            return Binding::Unresolved;
        }
        // A conflicting declaration/import at the same scope is not a safe guess.
        if local_width.is_some() && local_width == import_width {
            return Binding::Unresolved;
        }
        if local_width.is_some_and(|s| import_width.is_none_or(|i| s < i)) {
            return if local.len() == 1 {
                Binding::Local(local[0])
            } else {
                Binding::Unresolved
            };
        }
        if !imports.is_empty() {
            if imports.len() != 1 {
                return Binding::Unresolved;
            }
            let (index, binding) = imports[0];
            if binding.namespace != site.qualifier.is_some() {
                return Binding::Unresolved;
            }
            return Binding::Import {
                index,
                exported: if binding.namespace {
                    &site.name
                } else {
                    &binding.imported
                },
            };
        }
        if site.qualifier.is_some() {
            Binding::Unresolved
        } else {
            Binding::Unbound
        }
    }
}
