use clap::ValueEnum;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SecretPolicy {
    #[default]
    Redact,
    Fail,
    Allow,
}

#[derive(Debug, Clone)]
pub struct SecretFinding {
    pub kind: &'static str,
    pub start: usize,
    pub end: usize,
}

fn patterns() -> &'static Vec<(&'static str, Regex)> {
    static PATTERNS: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            (
                "private-key",
                Regex::new(r"(?s)-----BEGIN [^-\r\n]*PRIVATE KEY-----.*?-----END [^-\r\n]*PRIVATE KEY-----")
                    .unwrap(),
            ),
            (
                "github-token",
                Regex::new(r"\bgh[pousr]_[A-Za-z0-9]{20,}\b").unwrap(),
            ),
            (
                "aws-access-key",
                Regex::new(r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b").unwrap(),
            ),
            (
                "bearer-token",
                Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{20,}").unwrap(),
            ),
            (
                "jwt",
                Regex::new(r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b")
                    .unwrap(),
            ),
            (
                "credential-url",
                Regex::new(r"(?i)\b[a-z][a-z0-9+.-]*://[^\s/:@]{1,128}:[^\s/@]{8,}@")
                    .unwrap(),
            ),
            (
                "named-secret",
                Regex::new(
                    r#"(?i)\b(?:api[_-]?key|client[_-]?secret|access[_-]?token|auth[_-]?token|secret|password|passwd|token)\b[^\r\n=]{0,64}=\s*['"]?[A-Za-z0-9_./+~=-]{16,}['"]?"#,
                )
                .unwrap(),
            ),
        ]
    })
}

pub fn find_secrets(text: &str) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    for (kind, regex) in patterns() {
        for m in regex.find_iter(text) {
            findings.push(SecretFinding {
                kind,
                start: m.start(),
                end: m.end(),
            });
        }
    }
    findings.sort_by_key(|f| (f.start, f.end));
    findings
}

pub fn redact_secrets(text: &str) -> (String, usize) {
    let findings = find_secrets(text);
    if findings.is_empty() {
        return (text.to_string(), 0);
    }

    // Merge overlaps so a token matched by two patterns is replaced once.
    let mut ranges: Vec<(usize, usize)> = findings.iter().map(|f| (f.start, f.end)).collect();
    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }

    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end) in &merged {
        out.push_str(&text[cursor..*start]);
        out.push_str("[REDACTED_SECRET]");
        cursor = *end;
    }
    out.push_str(&text[cursor..]);
    (out, merged.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_named_secret() {
        // Build the fixture at runtime so source scanners do not mistake a test value
        // for a real credential and rewrite the source artifact.
        let secret = "a".repeat(24);
        let input = format!(r#"API_KEY = "{}""#, secret);
        let (out, count) = redact_secrets(&input);
        assert_eq!(count, 1);
        assert!(out.contains("[REDACTED_SECRET]"));
        assert!(!out.contains(&secret));
    }

    #[test]
    fn catches_github_token() {
        let token = format!("ghp_{}", "a".repeat(24));
        let input = format!("token={}", token);
        assert!(!find_secrets(&input).is_empty());
    }

    #[test]
    fn leaves_short_values_alone() {
        let input = r#"API_KEY = "short""#;
        let (out, count) = redact_secrets(input);
        assert_eq!(count, 0);
        assert_eq!(out, input);
    }
}
