use crate::security;
use tree_sitter::{Language, Node, Parser as TsParser, Range as TsRange};

pub fn redact_secrets(text: &str) -> String {
    security::redact_secrets(text).0
}

fn collect_comment_ranges(node: Node, ranges: &mut Vec<TsRange>) {
    let kind = node.kind();
    if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
        ranges.push(node.range());
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_comment_ranges(child, ranges);
        }
    }
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
        let mut ranges = Vec::new();
        collect_comment_ranges(tree.root_node(), &mut ranges);
        remove_byte_ranges(text, ranges)
    } else {
        text.to_string()
    }
}

pub fn minify_code(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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
    fn minify_blank_lines() {
        let code = "fn a() {}\n   \n\nfn b() {}   \n";
        assert_eq!(minify_code(code), "fn a() {}\nfn b() {}");
    }
}
