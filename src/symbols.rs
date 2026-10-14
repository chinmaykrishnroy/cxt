use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
    /// Exact AST identifiers, excluding comments, strings and import syntax.
    pub references: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceIndex {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<SourceSpan>,
    pub identifiers: BTreeSet<String>,
    pub parsed: bool,
}

fn span(node: Node) -> SourceSpan {
    SourceSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
    }
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
        index.imports.push(span(node));
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
                references: names,
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
    for _ in 0..2 {
        let identifiers: BTreeSet<_> = chosen
            .iter()
            .flat_map(|s| s.references.iter().cloned())
            .collect();
        for candidate in &index.symbols {
            if chosen.len() >= 24 {
                break;
            }
            let tokens = counter.count(&source[candidate.span.start_byte..candidate.span.end_byte]);
            if !chosen.iter().any(|s| s.id == candidate.id)
                && identifiers.contains(&candidate.name)
                && budget.is_none_or(|limit| cost.saturating_add(tokens) <= limit)
            {
                chosen.push(candidate);
                cost = cost.saturating_add(tokens);
            }
        }
    }
    let mut ranges = index.imports.clone();
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
