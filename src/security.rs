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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretScanner {
    #[default]
    Builtin,
    Gitleaks,
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
            ("structured-secret", Regex::new(r#"(?i)\b(?:api[_-]?key|client[_-]?secret|access[_-]?token|auth[_-]?token|secret|password|passwd|token)\b["']?\s*:\s*["']?[A-Za-z0-9_./+~=$!%&*-]{16,}["']?"#).unwrap()),
            ("github-fine-grained", Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{20,}\b").unwrap()),
            ("provider-token", Regex::new(r"\b(?:npm_[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{20,}|[sr]k_live_[A-Za-z0-9]{20,})\b").unwrap()),
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
    redact_findings(text, &findings)
}

pub fn configured_findings(
    text: &str,
    scanner: SecretScanner,
    allowlist: &[String],
) -> Result<Vec<SecretFinding>, String> {
    let rules: Vec<_> = allowlist
        .iter()
        .map(|r| Regex::new(r).map_err(|_| "invalid secret allowlist regex".to_string()))
        .collect::<Result<_, _>>()?;
    let mut findings = find_secrets(text);
    if scanner == SecretScanner::Gitleaks {
        findings.extend(gitleaks_findings(text)?);
    }
    findings.retain(|f| !rules.iter().any(|r| r.is_match(&text[f.start..f.end])));
    findings.sort_by_key(|f| (f.start, f.end));
    Ok(findings)
}

fn gitleaks_findings(text: &str) -> Result<Vec<SecretFinding>, String> {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    let dir = tempfile::tempdir().map_err(|_| "cannot create private scanner report directory")?;
    let report = dir.path().join("report.json");
    let mut command = Command::new("gitleaks");
    command
        .args([
            "stdin",
            "--no-banner",
            "--no-color",
            "--redact=100",
            "--exit-code",
            "10",
            "--timeout",
            "10",
            "--report-format",
            "json",
            "--report-path",
        ])
        .arg(&report)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|_| "gitleaks is unavailable; install it or select --secret-scanner builtin")?;
    let mut input = child.stdin.take().ok_or("scanner stdin unavailable")?;
    let source = text.to_owned();
    let writer = std::thread::spawn(move || input.write_all(source.as_bytes()));
    let started = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < std::time::Duration::from_secs(12) => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("gitleaks process failed or timed out; export blocked".into());
            }
        }
    };
    if !writer.is_finished() {
        // Closing/killing the child releases the pipe; do not wait on an unresponsive helper.
        return Err("gitleaks did not consume its input; export blocked".into());
    }
    if !matches!(writer.join(), Ok(Ok(()))) {
        return Err("gitleaks input failed; export blocked".into());
    }
    if !matches!(status.code(), Some(0 | 10)) {
        return Err("gitleaks scan failed; export blocked".into());
    }
    let mut bytes = Vec::new();
    if let Ok(file) = std::fs::File::open(&report) {
        file.take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "scanner report read failed")?;
    }
    if bytes.is_empty() && status.success() {
        return Ok(Vec::new());
    }
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("scanner report exceeded limit".into());
    }
    let report: Vec<serde_json::Value> =
        serde_json::from_slice(&bytes).map_err(|_| "invalid scanner report; export blocked")?;
    let mut offsets = vec![0usize];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    let mut findings = Vec::new();
    for finding in report {
        let start = finding["StartLine"]
            .as_u64()
            .ok_or("missing scanner location")? as usize;
        let end = finding["EndLine"]
            .as_u64()
            .ok_or("missing scanner location")? as usize;
        if start == 0 || end < start || end > offsets.len() {
            return Err("invalid scanner location; export blocked".into());
        }
        findings.push(SecretFinding {
            kind: "gitleaks-line",
            start: offsets[start - 1],
            end: offsets.get(end).copied().unwrap_or(text.len()),
        });
    }
    if status.code() == Some(10) && findings.is_empty() {
        return Err("scanner reported findings without locations; export blocked".into());
    }
    Ok(findings)
}

pub fn redact_findings(text: &str, findings: &[SecretFinding]) -> (String, usize) {
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
