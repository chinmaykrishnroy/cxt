use regex::Regex;
use tree_sitter::{Language, Node, Parser as TsParser, Range as TsRange};

pub fn redact_secrets(text: &str) -> String {
    // Matches common API key patterns, high-entropy secrets, and standard password structures
    let re = Regex::new(r"(?i)(api[_-]?key|secret|password|token|sk-ant-api|ghp_).*?['\x22][A-Za-z0-9\-_\.]{16,}['\x22]").unwrap();
    re.replace_all(text, "[REDACTED_SECRET]").to_string()
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

    // Merge overlapping/adjacent ranges so we don't double-remove bytes.
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
        .map(|line| line.trim_end()) // Remove trailing whitespace
        .filter(|line| !line.is_empty()) // Remove pure empty lines
        .collect::<Vec<&str>>()
        .join("\n")
}

pub fn count_tokens(text: &str) -> usize {
    // A fast, standard heuristic: 1 token ≈ 4 characters of code.
    text.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_LANG: fn() -> Language = tree_sitter_rust::language;

    #[test]
    fn redact_api_key() {
        let input = r#"const API_KEY: &str = "abcdef1234567890abcdef";"#;
        let out = redact_secrets(input);
        assert!(out.contains("[REDACTED_SECRET]"));
        assert!(!out.contains("abcdef1234567890abcdef"));
    }

    #[test]
    fn redact_password() {
        let input = r#"const PASSWORD: &str = "hunter2hunter2hunter2";"#;
        let out = redact_secrets(input);
        assert!(out.contains("[REDACTED_SECRET]"));
        assert!(!out.contains("hunter2hunter2hunter2"));
    }

    #[test]
    fn redact_ghp_token() {
        let input = r#"const TOKEN: &str = "ghp_abcdefghijklmnopqrstuv";"#;
        let out = redact_secrets(input);
        assert!(out.contains("[REDACTED_SECRET]"));
        assert!(!out.contains("ghp_abcdefghijklmnopqrstuv"));
    }

    #[test]
    fn short_secret_not_redacted() {
        let input = r#"const API_KEY: &str = "short";"#;
        let out = redact_secrets(input);
        assert!(!out.contains("[REDACTED_SECRET]"));
        assert!(out.contains("short"));
    }

    #[test]
    fn normal_string_not_redacted() {
        let input = r#"const GREETING: &str = "hello world";"#;
        let out = redact_secrets(input);
        assert!(!out.contains("[REDACTED_SECRET]"));
        assert!(out.contains("hello world"));
    }

    #[test]
    fn strip_rust_comments() {
        let code = "// secret\nfn main() { /* inline */ println!(\"hi\"); }\n";
        let out = strip_comments(code, RUST_LANG());
        assert!(!out.contains("// secret"));
        assert!(!out.contains("/* inline */"));
        assert!(out.contains("fn main()"));
        assert!(out.contains(r#"println!("hi")"#));
    }

    #[test]
    fn strip_rust_doc_comments() {
        let code = "/// docs\n/// more docs\nfn foo() {}\n";
        let out = strip_comments(code, RUST_LANG());
        assert!(!out.contains("///"));
        assert!(out.contains("fn foo()"));
    }

    #[test]
    fn minify_collapses_blank_lines_and_trims_whitespace() {
        let code = "fn a() {}\n   \n\nfn b() {}   \n";
        let out = minify_code(code);
        assert_eq!(out, "fn a() {}\nfn b() {}");
    }

    #[test]
    fn count_tokens_uses_len_over_four() {
        assert_eq!(count_tokens("abcd"), 1);
        assert_eq!(count_tokens("abcdefgh"), 2);
    }
}
