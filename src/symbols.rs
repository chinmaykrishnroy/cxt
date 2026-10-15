use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextLevel {
    #[default]
    Complete,
    Map,
    Focused,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub signature: String,
    pub span: SourceSpan,
    pub binding_scope: SourceSpan,
    /// Exact AST identifiers, excluding comments, strings and import syntax.
    pub references: BTreeSet<String>,
    pub reference_sites: Vec<ReferenceSite>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceIndex {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<SourceSpan>,
    pub import_scopes: BTreeMap<usize, SourceSpan>,
    pub identifiers: BTreeSet<String>,
    pub parsed: bool,
    pub bindings: Vec<ImportBinding>,
    pub exports: BTreeMap<String, String>,
    pub local_bindings: Vec<LocalBinding>,
    pub non_lexical_scopes: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalBinding {
    pub name: String,
    pub scope: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBinding {
    pub local: String,
    pub imported: String,
    pub module: String,
    pub scope: SourceSpan,
    pub namespace: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReferenceKind {
    Calls,
    References,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceSite {
    pub name: String,
    pub qualifier: Option<String>,
    pub span: SourceSpan,
    pub kind: ReferenceKind,
}

fn python_bindings(node: Node, source: &str, index: &mut SourceIndex) {
    let module = node.child_by_field_name("module_name");
    let mut owner = node;
    while let Some(parent) = owner.parent() {
        owner = parent;
        if matches!(owner.kind(), "function_definition" | "class_definition") {
            if let Some(wrapper) = owner
                .parent()
                .filter(|p| p.kind() == "decorated_definition")
            {
                owner = wrapper;
            }
            break;
        }
    }
    let mut cursor = node.walk();
    for name in node.children_by_field_name("name", &mut cursor) {
        let original = name.child_by_field_name("name").unwrap_or(name);
        let local = name.child_by_field_name("alias").unwrap_or(original);
        let namespace = module.is_none();
        // `import a.b` binds a, not b. Member paths are resolved conservatively.
        let local_text = &source[local.byte_range()];
        index.bindings.push(ImportBinding {
            local: crate::security::redact_secrets(if namespace && local == original {
                local_text.split('.').next().unwrap_or(local_text)
            } else {
                local_text
            })
            .0,
            imported: crate::security::redact_secrets(&source[original.byte_range()]).0,
            module: crate::security::redact_secrets(
                &source[module.unwrap_or(original).byte_range()],
            )
            .0,
            scope: span(owner),
            namespace,
        });
    }
}

fn javascript_bindings(node: Node, source: &str, index: &mut SourceIndex) {
    let Some(module) = node.child_by_field_name("source") else {
        return;
    };
    let text = &source[module.byte_range()];
    if text.len() < 2 || text.contains('\\') {
        return;
    }
    let module = crate::security::redact_secrets(&text[1..text.len() - 1]).0;
    let mut owner = node;
    while let Some(parent) = owner.parent() {
        owner = parent;
    }
    fn collect(node: Node, module: &str, owner: Node, source: &str, index: &mut SourceIndex) {
        if node.kind() == "identifier" && node.parent().is_some_and(|p| p.kind() == "import_clause")
        {
            index.bindings.push(ImportBinding {
                local: crate::security::redact_secrets(&source[node.byte_range()]).0,
                imported: "default".into(),
                module: module.into(),
                scope: span(owner),
                namespace: false,
            });
            return;
        }
        if node.kind() == "namespace_import" {
            let mut cursor = node.walk();
            if let Some(alias) = node
                .named_children(&mut cursor)
                .find(|n| n.kind() == "identifier")
            {
                index.bindings.push(ImportBinding {
                    local: crate::security::redact_secrets(&source[alias.byte_range()]).0,
                    imported: String::new(),
                    module: module.into(),
                    scope: span(owner),
                    namespace: true,
                });
            }
            return;
        }
        if node.kind() == "import_specifier" {
            if let Some(name) = node.child_by_field_name("name") {
                let alias = node.child_by_field_name("alias").unwrap_or(name);
                index.bindings.push(ImportBinding {
                    local: crate::security::redact_secrets(&source[alias.byte_range()]).0,
                    imported: crate::security::redact_secrets(&source[name.byte_range()]).0,
                    module: module.into(),
                    scope: span(owner),
                    namespace: false,
                });
            }
            return;
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            collect(child, module, owner, source, index);
        }
    }
    let mut cursor = node.walk();
    for clause in node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "import_clause")
    {
        collect(clause, &module, owner, source, index);
    }
}

fn span(node: Node) -> SourceSpan {
    SourceSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
    }
}

fn binding_scope(node: Node) -> SourceSpan {
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let python = root.kind() == "module";
    let mut owner = node.parent();
    let function_scoped = node
        .parent()
        .is_some_and(|p| p.kind() == "variable_declaration");
    while let Some(parent) = owner {
        if matches!(
            parent.kind(),
            "function_definition"
                | "function_declaration"
                | "function_item"
                | "arrow_function"
                | "function_expression"
                | "method_definition"
                | "class_definition"
        ) || (!python
            && !function_scoped
            && matches!(
                parent.kind(),
                "statement_block" | "block" | "for_statement" | "for_in_statement" | "switch_body"
            ))
        {
            return span(parent);
        }
        owner = parent.parent();
    }
    span(root)
}

fn import_scope(mut node: Node) -> SourceSpan {
    while let Some(parent) = node.parent() {
        node = parent;
        if matches!(
            node.kind(),
            "function_item"
                | "function_definition"
                | "function_declaration"
                | "method_definition"
                | "method_declaration"
                | "mod_item"
                | "class_definition"
                | "class_declaration"
                | "namespace_definition"
                | "block"
                | "statement_block"
        ) {
            break;
        }
    }
    span(node)
}

fn identifier(node: Node, source: &str) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "field_identifier" | "constant"
    ) {
        return Some(source[node.byte_range()].to_string());
    }
    if let Some(name) = node.child_by_field_name("name") {
        return Some(source[name.byte_range()].to_string());
    }
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return identifier(declarator, source);
    }
    None
}

fn references(node: Node, source: &str, names: &mut BTreeSet<String>) {
    let kind = node.kind();
    if kind.contains("comment")
        || kind.contains("string")
        || kind.contains("literal")
        || matches!(
            kind,
            "use_declaration"
                | "import_statement"
                | "import_from_statement"
                | "import_declaration"
                | "preproc_include"
                | "using_directive"
                | "package_clause"
        )
    {
        return;
    }
    if kind.contains("identifier") || kind == "constant" {
        names.insert(source[node.byte_range()].to_string());
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            references(child, source, names);
        }
    }
}

fn reference_sites(root: Node, source: &str) -> Vec<ReferenceSite> {
    fn called(node: Node) -> bool {
        node.parent().is_some_and(|parent| {
            matches!(parent.kind(), "call" | "call_expression")
                && parent.child_by_field_name("function") == Some(node)
        })
    }
    fn walk(node: Node, root: Node, source: &str, sites: &mut Vec<ReferenceSite>) {
        let kind = node.kind();
        if kind.contains("comment")
            || kind.contains("string")
            || kind.contains("literal")
            || matches!(
                kind,
                "parameters"
                    | "formal_parameters"
                    | "parameter_list"
                    | "import_statement"
                    | "import_from_statement"
                    | "use_declaration"
                    | "import_declaration"
                    | "type_annotation"
            )
            || (node != root
                && matches!(
                    kind,
                    "function_definition"
                        | "function_declaration"
                        | "function_item"
                        | "method_definition"
                        | "class_definition"
                        | "class_declaration"
                ))
        {
            return;
        }
        let member = match kind {
            "attribute" => node
                .child_by_field_name("object")
                .zip(node.child_by_field_name("attribute")),
            "member_expression" | "field_expression" => node
                .child_by_field_name("object")
                .or_else(|| node.child_by_field_name("value"))
                .zip(
                    node.child_by_field_name("property")
                        .or_else(|| node.child_by_field_name("field")),
                ),
            "scoped_identifier" => node
                .child_by_field_name("path")
                .zip(node.child_by_field_name("name")),
            _ => None,
        };
        if let Some((object, name)) = member {
            let qualifier = &source[object.byte_range()];
            if qualifier
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | ':'))
            {
                sites.push(ReferenceSite {
                    name: crate::security::redact_secrets(&source[name.byte_range()]).0,
                    qualifier: Some(crate::security::redact_secrets(qualifier).0),
                    span: span(name),
                    kind: if called(node) {
                        ReferenceKind::Calls
                    } else {
                        ReferenceKind::References
                    },
                });
            }
            // A property on an unknown receiver must never become a bare-name match.
            return;
        }
        if kind.contains("identifier") || kind == "constant" {
            let declaration_name = node.parent().is_some_and(|parent| {
                parent.child_by_field_name("name") == Some(node)
                    && !matches!(parent.kind(), "call" | "call_expression")
            });
            if !declaration_name {
                sites.push(ReferenceSite {
                    name: crate::security::redact_secrets(&source[node.byte_range()]).0,
                    qualifier: None,
                    span: span(node),
                    kind: if called(node) {
                        ReferenceKind::Calls
                    } else {
                        ReferenceKind::References
                    },
                });
            }
            return;
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(child, root, source, sites);
        }
    }
    let mut sites = Vec::new();
    walk(root, root, source, &mut sites);
    sites
}

fn parameter_names(node: Node, source: &str, names: &mut BTreeSet<String>) {
    if matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier_pattern"
    ) {
        names.insert(crate::security::redact_secrets(&source[node.byte_range()]).0);
        return;
    }
    if matches!(node.kind(), "type" | "type_annotation") {
        return;
    }
    if matches!(
        node.kind(),
        "default_parameter" | "typed_default_parameter" | "assignment_pattern"
    ) {
        if let Some(pattern) = node
            .child_by_field_name("name")
            .or_else(|| node.child_by_field_name("left"))
        {
            parameter_names(pattern, source, names);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        parameter_names(child, source, names);
    }
}

fn python_assignment_names(root: Node, source: &str, names: &mut BTreeSet<String>) {
    fn pattern(node: Node, source: &str, names: &mut BTreeSet<String>) {
        if node.kind() == "identifier" {
            names.insert(crate::security::redact_secrets(&source[node.byte_range()]).0);
        } else if matches!(
            node.kind(),
            "pattern_list"
                | "tuple_pattern"
                | "list_pattern"
                | "tuple"
                | "list"
                | "list_splat_pattern"
        ) {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                pattern(child, source, names);
            }
        }
    }
    fn walk(node: Node, root: Node, source: &str, names: &mut BTreeSet<String>) {
        if node != root
            && matches!(
                node.kind(),
                "function_definition"
                    | "class_definition"
                    | "lambda"
                    | "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            )
        {
            return;
        }
        if matches!(
            node.kind(),
            "assignment" | "augmented_assignment" | "for_statement" | "named_expression"
        ) {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.child_by_field_name("name"))
            {
                pattern(left, source, names);
            }
        }
        if node.kind() == "as_pattern" {
            if let Some(alias) = node.child_by_field_name("alias") {
                pattern(alias, source, names);
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(child, root, source, names);
        }
    }
    if root.kind() == "function_definition"
        && root
            .child_by_field_name("body")
            .is_some_and(|body| body.kind() == "block")
    {
        walk(root, root, source, names);
    }
}

fn visit(node: Node, source: &str, parents: &[String], index: &mut SourceIndex) {
    let kind = node.kind();
    if kind.contains("comment") || kind.contains("string") || kind.contains("literal") {
        return;
    }
    if kind.contains("identifier") || kind == "constant" {
        index
            .identifiers
            .insert(source[node.byte_range()].to_string());
    }
    if let Some(parameters) = node.child_by_field_name("parameters") {
        let mut names = BTreeSet::new();
        parameter_names(parameters, source, &mut names);
        python_assignment_names(node, source, &mut names);
        index
            .local_bindings
            .extend(names.into_iter().map(|name| LocalBinding {
                name,
                scope: span(node),
            }));
    }
    if matches!(
        kind,
        "use_declaration"
            | "import_statement"
            | "import_from_statement"
            | "import_declaration"
            | "preproc_include"
            | "using_directive"
            | "package_clause"
    ) {
        if kind == "import_from_statement"
            || (kind == "import_statement" && node.child_by_field_name("source").is_none())
        {
            python_bindings(node, source, index);
        } else if kind == "import_statement" && node.child_by_field_name("source").is_some() {
            javascript_bindings(node, source, index);
        }
        index.imports.push(span(node));
        index
            .import_scopes
            .insert(node.start_byte(), import_scope(node));
        return;
    }
    let definition = matches!(
        kind,
        "function_item"
            | "function_definition"
            | "function_declaration"
            | "method_definition"
            | "method_declaration"
            | "constructor_declaration"
            | "method"
            | "singleton_method"
            | "class_declaration"
            | "class_definition"
            | "class"
            | "struct_item"
            | "enum_item"
            | "interface_declaration"
            | "type_alias_declaration"
            | "type_item"
            | "type_spec"
            | "const_item"
            | "static_item"
            | "variable_declarator"
    ) || (kind == "field_declaration"
        && node
            .parent()
            .is_some_and(|parent| parent.kind() == "field_declaration_list"));
    let mut scope = parents.to_vec();
    if kind == "impl_item" {
        if let Some(mut owner) = node.child_by_field_name("type") {
            while owner.kind() == "generic_type" {
                let Some(base) = owner.child_by_field_name("type") else {
                    break;
                };
                owner = base;
            }
            scope.push(source[owner.byte_range()].to_string());
            index
                .non_lexical_scopes
                .insert(crate::security::redact_secrets(&scope.join("::")).0);
        }
    }
    if definition {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| source.get(n.byte_range()).map(str::to_string))
            .or_else(|| identifier(node, source));
        if let Some(name) = name {
            let mut declaration = node;
            while let Some(parent) = declaration.parent() {
                if !matches!(
                    parent.kind(),
                    "export_statement"
                        | "decorated_definition"
                        | "lexical_declaration"
                        | "variable_declaration"
                ) {
                    break;
                }
                declaration = parent;
            }
            let qualified_name = parents
                .iter()
                .chain(std::iter::once(&name))
                .cloned()
                .collect::<Vec<_>>()
                .join("::");
            if parents.is_empty() && declaration.kind() == "export_statement" {
                let default = (0..declaration.child_count())
                    .any(|i| declaration.child(i).is_some_and(|c| c.kind() == "default"));
                let exported = if default { "default" } else { &name };
                index.exports.insert(
                    crate::security::redact_secrets(exported).0,
                    crate::security::redact_secrets(&name).0,
                );
            }
            let body = node
                .child_by_field_name("body")
                .map(|n| n.start_byte())
                .unwrap_or(node.end_byte());
            let signature = source[declaration.start_byte()..body]
                .split('=')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(240)
                .collect::<String>();
            let mut names = BTreeSet::new();
            references(node, source, &mut names);
            names.remove(&name);
            let mut source_span = span(declaration);
            if kind == "field_declaration" {
                if let Some(separator) =
                    declaration.next_sibling().filter(|item| item.kind() == ",")
                {
                    source_span.end_byte = separator.end_byte();
                    source_span.end_line = separator.end_position().row + 1;
                }
            }
            if matches!(
                kind,
                "field_declaration"
                    | "function_item"
                    | "struct_item"
                    | "enum_item"
                    | "const_item"
                    | "static_item"
                    | "type_item"
            ) {
                let mut previous = declaration.prev_named_sibling();
                while let Some(item) = previous {
                    if !matches!(
                        item.kind(),
                        "attribute_item" | "line_comment" | "block_comment"
                    ) {
                        break;
                    }
                    source_span.start_byte = item.start_byte();
                    source_span.start_line = item.start_position().row + 1;
                    previous = item.prev_named_sibling();
                }
            }
            index.symbols.push(Symbol {
                id: format!("{}:{}:{}", kind, qualified_name, node.start_byte()),
                name: name.clone(),
                qualified_name,
                kind: kind.into(),
                signature,
                span: source_span,
                binding_scope: binding_scope(node),
                references: names,
                reference_sites: reference_sites(node, source),
            });
            scope.push(name);
        }
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            visit(child, source, &scope, index);
        }
    }
}

pub fn extract(path: &Path, source: &str) -> SourceIndex {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let Some(language) = crate::tracer::get_language(ext) else {
        return SourceIndex::default();
    };
    let mut parser = Parser::new();
    if parser.set_language(language).is_err() {
        return SourceIndex::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return SourceIndex::default();
    };
    if tree.root_node().has_error() {
        return SourceIndex::default();
    }
    let mut index = SourceIndex {
        parsed: true,
        ..Default::default()
    };
    visit(tree.root_node(), source, &[], &mut index);
    use sha2::{Digest, Sha256};
    for symbol in &mut index.symbols {
        symbol.id = format!(
            "{:x}",
            Sha256::digest(format!("{}:{}", path.display(), symbol.id).as_bytes())
        );
        symbol.name = crate::security::redact_secrets(&symbol.name).0;
        symbol.qualified_name = crate::security::redact_secrets(&symbol.qualified_name).0;
        symbol.signature = crate::security::redact_secrets(&symbol.signature).0;
        symbol.references = symbol
            .references
            .iter()
            .map(|name| crate::security::redact_secrets(name).0)
            .collect();
    }
    index.identifiers = index
        .identifiers
        .into_iter()
        .map(|name| crate::security::redact_secrets(&name).0)
        .collect();
    index
}

pub fn words(text: &str) -> Vec<String> {
    let mut expanded = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_uppercase()
            && i > 0
            && (chars[i - 1].is_lowercase()
                || (chars[i - 1].is_uppercase()
                    && chars.get(i + 1).is_some_and(|n| n.is_lowercase())))
        {
            expanded.push(' ');
        }
        expanded.extend(c.to_lowercase());
    }
    expanded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn represent(
    index: &SourceIndex,
    source: &str,
    level: ContextLevel,
    query: Option<&str>,
    symbol: Option<&str>,
) -> Result<(String, Vec<SourceSpan>, String), String> {
    represent_bounded(
        index,
        source,
        level,
        query,
        symbol,
        &crate::token::TokenCounter::for_model("heuristic"),
        None,
    )
}

pub fn represent_bounded(
    index: &SourceIndex,
    source: &str,
    level: ContextLevel,
    query: Option<&str>,
    symbol: Option<&str>,
    counter: &crate::token::TokenCounter,
    budget: Option<usize>,
) -> Result<(String, Vec<SourceSpan>, String), String> {
    represent_with_hints(
        index,
        source,
        level,
        query,
        symbol,
        counter,
        budget,
        &BTreeSet::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn represent_with_hints(
    index: &SourceIndex,
    source: &str,
    level: ContextLevel,
    query: Option<&str>,
    symbol: Option<&str>,
    counter: &crate::token::TokenCounter,
    budget: Option<usize>,
    hints: &BTreeSet<String>,
) -> Result<(String, Vec<SourceSpan>, String), String> {
    if level == ContextLevel::Complete && symbol.is_none() {
        return Ok((source.into(), Vec::new(), "complete".into()));
    }
    if !index.parsed || index.symbols.is_empty() {
        if symbol.is_some() {
            return Err("requested symbol is unavailable in this grammar or parse".into());
        }
        return Ok((source.into(), Vec::new(), "complete-fallback".into()));
    }
    if level == ContextLevel::Map {
        let content = index
            .symbols
            .iter()
            .map(|s| {
                format!(
                    "{}:{}-{} {} {}",
                    s.kind, s.span.start_line, s.span.end_line, s.qualified_name, s.signature
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Ok((
            content,
            index.symbols.iter().map(|s| s.span.clone()).collect(),
            "map".into(),
        ));
    }
    let terms = query.map(words).unwrap_or_default();
    let hint_weight = |s: &Symbol| usize::from(hints.contains(&s.qualified_name)) * 100;
    let mut scored: Vec<_> = index
        .symbols
        .iter()
        .filter_map(|s| {
            let score = if let Some(name) = symbol {
                if s.name == name || s.qualified_name == name {
                    1000
                } else {
                    0
                }
            } else {
                let tokens = words(&source[s.span.start_byte..s.span.end_byte]);
                let names = words(&s.qualified_name);
                hint_weight(s)
                    + terms.iter().filter(|t| tokens.contains(t)).count()
                    + 10 * terms.iter().filter(|t| names.contains(t)).count()
            };
            (score > 0 || (symbol.is_none() && terms.is_empty())).then_some((score, s))
        })
        .collect();
    if scored.is_empty() {
        if symbol.is_some() {
            return Err("requested symbol was not found".into());
        }
        return Ok((source.into(), Vec::new(), "complete-fallback".into()));
    }
    scored.sort_by(|(a, sa), (b, sb)| {
        b.cmp(a)
            .then_with(|| {
                (sa.span.end_byte - sa.span.start_byte)
                    .cmp(&(sb.span.end_byte - sb.span.start_byte))
            })
            .then_with(|| sa.span.start_byte.cmp(&sb.span.start_byte))
    });
    if symbol.is_none() && !terms.is_empty() {
        let lexical_max = scored
            .iter()
            .map(|(score, s)| score - hint_weight(s))
            .max()
            .unwrap_or(0);
        let cutoff = (lexical_max / 3).max(1);
        scored.retain(|(score, s)| hint_weight(s) > 0 || *score >= cutoff);
    }
    let mut cost = 0usize;
    let mut chosen: Vec<&Symbol> = Vec::new();
    for (_, item) in &scored {
        if chosen.len() >= if symbol.is_some() { 16 } else { 8 } {
            break;
        }
        let tokens = counter.count(&source[item.span.start_byte..item.span.end_byte]);
        if symbol.is_some() || budget.is_none_or(|limit| cost.saturating_add(tokens) <= limit) {
            chosen.push(item);
            cost = cost.saturating_add(tokens);
        }
    }
    if chosen.is_empty() {
        if level == ContextLevel::Adaptive {
            return represent(index, source, ContextLevel::Map, None, None);
        }
        return Err("no relevant implementation span fits the token budget; increase it or use map/adaptive context".into());
    }
    let bindings = crate::bindings::ScopeIndex::new(index);
    for _ in 0..2 {
        let helpers: BTreeSet<_> = chosen
            .iter()
            .flat_map(|s| {
                s.reference_sites
                    .iter()
                    .filter_map(|site| match bindings.resolve(s, site) {
                        crate::bindings::Binding::Local(target) => Some(target.id.as_str()),
                        _ => None,
                    })
            })
            .collect();
        for candidate in &index.symbols {
            if chosen.len() >= 24 {
                break;
            }
            let tokens = counter.count(&source[candidate.span.start_byte..candidate.span.end_byte]);
            if !chosen.iter().any(|s| s.id == candidate.id)
                && helpers.contains(candidate.id.as_str())
                && budget.is_none_or(|limit| cost.saturating_add(tokens) <= limit)
            {
                chosen.push(candidate);
                cost = cost.saturating_add(tokens);
            }
        }
    }
    let mut ranges: Vec<_> = index
        .imports
        .iter()
        .filter(|import| {
            index
                .import_scopes
                .get(&import.start_byte)
                .is_none_or(|scope| {
                    chosen.iter().any(|symbol| {
                        scope.start_byte <= symbol.span.start_byte
                            && scope.end_byte >= symbol.span.end_byte
                    })
                })
        })
        .cloned()
        .collect();
    ranges.extend(chosen.iter().map(|s| s.span.clone()));
    ranges.sort_by_key(|s| s.start_byte);
    let mut merged: Vec<SourceSpan> = Vec::new();
    for range in ranges {
        if let Some(last) = merged.last_mut() {
            if range.start_byte <= last.end_byte {
                last.end_byte = last.end_byte.max(range.end_byte);
                last.end_line = last.end_line.max(range.end_line);
                continue;
            }
        }
        merged.push(range);
    }
    let content = merged
        .iter()
        .map(|s| &source[s.start_byte..s.end_byte])
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok((content, merged, "focused".into()))
}
