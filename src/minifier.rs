use crate::security;
use tree_sitter::{Language, Node, Parser as TsParser, Range as TsRange};

pub fn redact_secrets(text: &str) -> String {
    security::redact_secrets(text).0
}

fn collect_comment_ranges(node: Node, source: &str, ranges: &mut Vec<TsRange>) {
    let kind = node.kind();
    if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
        if !preserve_comment(&source[node.byte_range()]) {
            ranges.push(node.range());
        }
        return;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_comment_ranges(child, source, ranges);
        }
    }
}

fn preserve_comment(text: &str) -> bool {
    if text.starts_with("///")
        || text.starts_with("//!")
        || (text.starts_with("/**") && text != "/**/")
        || text.starts_with("/*!")
    {
        return true;
    }
    let lower = text.to_ascii_lowercase();
    [
        "@",
        "type:",
        "noqa",
        "nolint",
        "nosec",
        "eslint",
        "prettier",
        "go:",
        "coding",
        "encoding",
        "sourcemappingurl",
        "sourceurl",
        "copyright",
        "license",
        "istanbul",
        "c8 ",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn remove_byte_ranges(text: &str, mut ranges: Vec<TsRange>) -> String {
    if ranges.is_empty() {
        return text.to_string();
    }
    ranges.sort_by_key(|r| r.start_byte);
    let mut merged: Vec<TsRange> = Vec::new();
    for r in ranges {
        if let Some(last) = merged.last_mut() {
            if r.start_byte <= last.end_byte {
                last.end_byte = last.end_byte.max(r.end_byte);
                continue;
            }
        }
        merged.push(r);
    }

    let bytes = text.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut last_end = 0;
    for r in merged {
        result.extend_from_slice(&bytes[last_end..r.start_byte]);
        result.push(b' ');
        result.extend(
            bytes[r.start_byte..r.end_byte]
                .iter()
                .copied()
                .filter(|b| matches!(b, b'\r' | b'\n')),
        );
        last_end = r.end_byte;
    }
    result.extend_from_slice(&bytes[last_end..]);
    String::from_utf8(result).unwrap_or_else(|_| text.to_string())
}

pub fn strip_comments(text: &str, lang: Language) -> String {
    let mut parser = TsParser::new();
    if parser.set_language(lang).is_err() {
        return text.to_string();
    }
    if let Some(tree) = parser.parse(text, None) {
        if tree.root_node().has_error() {
            return text.to_string();
        }
        let mut ranges = Vec::new();
        collect_comment_ranges(tree.root_node(), text, &mut ranges);
        remove_byte_ranges(text, ranges)
    } else {
        text.to_string()
    }
}

/// Grammarless input is preserved; callers with a known grammar use `minify_syntax`.
pub fn minify_code(text: &str) -> String {
    // Without a grammar, whitespace may be literal content (or significant data).
    text.to_string()
}

/// Conservative source compaction; invalid parses fall back to the original text.
pub fn minify_syntax(text: &str, lang: Language) -> String {
    let mut parser = TsParser::new();
    if parser.set_language(lang).is_err() {
        return text.to_string();
    }
    let Some(tree) = parser.parse(text, None) else {
        return text.to_string();
    };
    if tree.root_node().has_error() {
        return text.to_string();
    }
    let stripped = strip_comments(text, lang);
    let Some(tree) = parser.parse(&stripped, None) else {
        return text.to_string();
    };
    if tree.root_node().has_error() {
        return text.to_string();
    }
    let mut protected = Vec::new();
    collect_protected_ranges(tree.root_node(), &mut protected);
    let mut out = String::with_capacity(stripped.len());
    let mut offset = 0;
    let mut protected_index = 0;
    for line in stripped.split_inclusive('\n') {
        let end = offset + line.len();
        while protected
            .get(protected_index)
            .is_some_and(|r| r.end_byte <= offset)
        {
            protected_index += 1;
        }
        if protected
            .get(protected_index)
            .is_some_and(|r| r.start_byte < end)
        {
            out.push_str(line);
        } else if !line.trim().is_empty() {
            let body = line.trim_end_matches(['\r', '\n']);
            out.push_str(body.trim_end_matches([' ', '\t']));
            out.push_str(&line[body.len()..]);
        }
        offset = end;
    }
    out
}

fn collect_protected_ranges(node: Node, ranges: &mut Vec<TsRange>) {
    let kind = node.kind();
    if node.child_count() == 0
        || kind.contains("string")
        || kind.contains("template")
        || kind.contains("literal")
        || kind.contains("heredoc")
        || kind.contains("comment")
        || kind.starts_with("preproc")
        || kind == "jsx_text"
    {
        ranges.push(node.range());
        return;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_protected_ranges(child, ranges);
        }
    }
}

pub fn count_tokens(text: &str) -> usize {
    crate::token::TokenCounter::for_model("heuristic").count(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_api_key() {
        let secret = "a".repeat(24);
        let input = format!(r#"const API_KEY: &str = "{}";"#, secret);
        let out = redact_secrets(&input);
        assert!(out.contains("[REDACTED_SECRET]"));
        assert!(!out.contains(&secret));
    }

    #[test]
    fn strip_rust_comments() {
        let code = "// comment\nfn main() { /* inline */ println!(\"hi\"); }\n";
        let out = strip_comments(code, tree_sitter_rust::language());
        assert!(!out.contains("// comment"));
        assert!(!out.contains("/* inline */"));
        assert!(out.contains("fn main()"));
    }

    #[test]
    fn grammarless_compaction_preserves_input() {
        let code = "fn a() {}\n   \n\nfn b() {}   \n";
        assert_eq!(minify_code(code), code);
    }
}
