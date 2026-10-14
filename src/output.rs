use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Markdown,
    Xml,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PathStyle {
    #[default]
    Relative,
    Absolute,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum Compression {
    None,
    Gzip,
    Zstd,
}

#[derive(Debug, Clone)]
pub struct PackedFile {
    pub path: PathBuf,
    pub content: String,
    pub language: String,
    pub tokens: usize,
    pub score: i64,
    pub reasons: Vec<String>,
    pub redactions: usize,
    pub spans: Vec<crate::symbols::SourceSpan>,
    pub representation: String,
    pub source_sha256: String,
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub format: OutputFormat,
    pub path_style: PathStyle,
    pub root: PathBuf,
    pub manifest: bool,
    pub model: String,
    pub estimated_tokens: usize,
    pub selection_mode: String,
    pub query: Option<String>,
    pub git_commit: Option<String>,
    pub redactions: usize,
    pub omissions: Vec<(PathBuf, String)>,
    pub budget_excluded: usize,
    pub skipped: usize,
}

#[derive(Serialize)]
struct JsonDocument<'a> {
    schema: &'static str,
    metadata: JsonMetadata<'a>,
    files: Vec<JsonFile<'a>>,
}

#[derive(Serialize)]
struct JsonMetadata<'a> {
    token_accounting: &'static str,
    generated_at_unix: u64,
    selection_mode: &'a str,
    query: Option<&'a str>,
    model: &'a str,
    estimated_tokens: usize,
    redactions: usize,
    git_commit: Option<&'a str>,
    budget_excluded: usize,
    skipped: usize,
    omitted_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    omissions: Vec<JsonOmission>,
}

#[derive(Serialize)]
struct JsonOmission {
    path: Option<String>,
    reason: String,
}

#[derive(Serialize)]
struct JsonFile<'a> {
    source_sha256: &'a str,
    spans: &'a [crate::symbols::SourceSpan],
    representation: &'a str,
    path: Option<String>,
    language: &'a str,
    estimated_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reasons: Vec<String>,
    content: &'a str,
}

pub fn render(files: &[PackedFile], options: &RenderOptions) -> Result<String, String> {
    match options.format {
        OutputFormat::Markdown => Ok(render_markdown(files, options)),
        OutputFormat::Xml => {
            validate_xml_text(&options.selection_mode)?;
            validate_xml_text(&options.model)?;
            if let Some(commit) = &options.git_commit {
                validate_xml_text(commit)?;
            }
            if options.manifest {
                for (path, reason) in options.omissions.iter().take(64) {
                    if let Some(path) = rendered_path(path, options) {
                        validate_xml_text(&path)?;
                    }
                    validate_xml_text(reason)?;
                }
            }
            for file in files {
                validate_xml_text(&file.content)?;
                validate_xml_text(&file.language)?;
                validate_xml_text(&file.representation)?;
                if let Some(path) = rendered_path(&file.path, options) {
                    validate_xml_text(&path)?;
                }
                if options.manifest {
                    for reason in &file.reasons {
                        validate_xml_text(reason)?;
                    }
                }
            }
            Ok(render_xml(files, options))
        }
        OutputFormat::Json => render_json(files, options),
    }
}

fn validate_xml_text(text: &str) -> Result<(), String> {
    if let Some(c) = text.chars().find(|c| !matches!(*c, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')) {
        return Err(format!("XML 1.0 cannot represent U+{:04X}; use JSON output to preserve this character", c as u32));
    }
    Ok(())
}

fn rendered_path(path: &Path, options: &RenderOptions) -> Option<String> {
    match options.path_style {
        PathStyle::None => None,
        PathStyle::Absolute => Some(
            path.canonicalize()
                .unwrap_or_else(|_| path.to_path_buf())
                .display()
                .to_string(),
        ),
        PathStyle::Relative => Some(
            path.strip_prefix(&options.root)
                .unwrap_or(path)
                .display()
                .to_string(),
        ),
    }
}

fn generated_at_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn sha256(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn render_markdown(files: &[PackedFile], options: &RenderOptions) -> String {
    let mut out = String::from("# Codebase Context\n\n");

    if options.manifest {
        out.push_str("## Context Manifest\n\n");
        out.push_str(&format!(
            "- Selection: {}\n",
            markdown_inline(&options.selection_mode)
        ));
        if let Some(query) = &options.query {
            out.push_str(&format!("- Query: {}\n", markdown_inline(query)));
        }
        out.push_str(&format!(
            "- Model profile: {}\n",
            markdown_inline(&options.model)
        ));
        out.push_str(&format!(
            "- {} tokens: `{}`\n",
            if crate::token::TokenCounter::for_model(&options.model).is_exact() {
                "Exact encoding"
            } else {
                "Estimated"
            },
            options.estimated_tokens
        ));
        out.push_str(&format!("- Secret redactions: `{}`\n", options.redactions));
        out.push_str(&format!(
            "- Excluded by budget: `{}`; skipped: `{}`\n",
            options.budget_excluded, options.skipped
        ));
        for (path, reason) in options.omissions.iter().take(64) {
            let path = rendered_path(path, options).unwrap_or_else(|| "(hidden path)".into());
            out.push_str(&format!(
                "- Omitted {}: {}\n",
                markdown_inline(&path),
                markdown_inline(reason)
            ));
        }
        if let Some(commit) = &options.git_commit {
            out.push_str(&format!("- Git commit: {}\n", markdown_inline(commit)));
        }
        out.push('\n');
    }

    for (index, file) in files.iter().enumerate() {
        if let Some(path) = rendered_path(&file.path, options) {
            out.push_str(&format!("### File: {}\n", markdown_inline(&path)));
        } else {
            out.push_str(&format!("### File {}\n", index + 1));
        }

        if options.manifest {
            out.push_str(&format!(
                "<!-- tokens={} score={} sha256={} reasons={} -->\n",
                file.tokens,
                file.score,
                sha256(&file.content),
                file.reasons
                    .join(" | ")
                    .replace("--", "—")
                    .replace(['\r', '\n', '<', '>'], " ")
            ));
        }

        if !file.spans.is_empty() {
            out.push_str(&format!(
                "Representation: {}; original source lines: {}.\n",
                markdown_inline(&file.representation),
                file.spans
                    .iter()
                    .map(|s| format!("{}-{}", s.start_line, s.end_line))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }

        let fence = "`".repeat(longest_backtick_run(&file.content).saturating_add(1).max(3));
        out.push_str(&format!(
            "{}{}\n{}\n{}\n\n",
            fence,
            file.language
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+'))
                .collect::<String>(),
            file.content,
            fence
        ));
    }

    out
}

fn markdown_inline(text: &str) -> String {
    let text: String = text
        .chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect();
    let delimiter = "`".repeat(longest_backtick_run(&text).saturating_add(1).max(1));
    if text.starts_with('`') || text.ends_with('`') || text.starts_with(' ') || text.ends_with(' ')
    {
        format!("{delimiter} {text} {delimiter}")
    } else {
        format!("{delimiter}{text}{delimiter}")
    }
}

fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for byte in text.bytes() {
        if byte == b'`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}

fn render_xml(files: &[PackedFile], options: &RenderOptions) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<context>\n");
    out.push_str(&format!(
        "  <metadata selection=\"{}\" model=\"{}\" estimated_tokens=\"{}\" redactions=\"{}\" budget_excluded=\"{}\" skipped=\"{}\" token_accounting=\"{}\"",
        xml_escape(&options.selection_mode),
        xml_escape(&options.model),
        options.estimated_tokens,
        options.redactions,
        options.budget_excluded,
        options.skipped,
        if crate::token::TokenCounter::for_model(&options.model).is_exact() { "exact-encoding" } else { "estimated" }
    ));
    if let Some(commit) = &options.git_commit {
        out.push_str(&format!(" git_commit=\"{}\"", xml_escape(commit)));
    }
    out.push_str(" />\n");
    if options.manifest {
        for (path, reason) in options.omissions.iter().take(64) {
            let path = rendered_path(path, options).unwrap_or_default();
            out.push_str(&format!(
                "  <omitted path=\"{}\">{}</omitted>\n",
                xml_escape(&path),
                xml_escape(reason)
            ));
        }
    }

    for file in files {
        let path = rendered_path(&file.path, options);
        out.push_str("  <file");
        if let Some(path) = path {
            out.push_str(&format!(" path=\"{}\"", xml_escape(&path)));
        }
        out.push_str(&format!(
            " language=\"{}\" estimated_tokens=\"{}\"",
            xml_escape(&file.language),
            file.tokens
        ));
        if options.manifest {
            out.push_str(&format!(" sha256=\"{}\"", sha256(&file.content)));
        }
        out.push_str(">\n");
        if !file.spans.is_empty() {
            out.push_str(&format!(
                "    <representation>{}</representation>\n    <spans>\n",
                xml_escape(&file.representation)
            ));
            for span in &file.spans {
                out.push_str(&format!("      <span start_byte=\"{}\" end_byte=\"{}\" start_line=\"{}\" end_line=\"{}\" />\n", span.start_byte, span.end_byte, span.start_line, span.end_line));
            }
            out.push_str("    </spans>\n");
        }
        if options.manifest && !file.reasons.is_empty() {
            out.push_str("    <reasons>\n");
            for reason in &file.reasons {
                out.push_str(&format!("      <reason>{}</reason>\n", xml_escape(reason)));
            }
            out.push_str("    </reasons>\n");
        }
        out.push_str(&format!(
            "    <content>{}</content>\n",
            xml_escape(&file.content)
        ));
        out.push_str("  </file>\n");
    }
    out.push_str("</context>\n");
    out
}

fn render_json(files: &[PackedFile], options: &RenderOptions) -> Result<String, String> {
    let json_files = files
        .iter()
        .map(|file| JsonFile {
            source_sha256: &file.source_sha256,
            spans: &file.spans,
            representation: &file.representation,
            path: rendered_path(&file.path, options),
            language: &file.language,
            estimated_tokens: file.tokens,
            sha256: options.manifest.then(|| sha256(&file.content)),
            reasons: if options.manifest {
                file.reasons.clone()
            } else {
                Vec::new()
            },
            content: &file.content,
        })
        .collect();

    let doc = JsonDocument {
        schema: "cxt.context.v1",
        metadata: JsonMetadata {
            token_accounting: if crate::token::TokenCounter::for_model(&options.model).is_exact() {
                "exact-encoding"
            } else {
                "estimated"
            },
            generated_at_unix: generated_at_unix(),
            selection_mode: &options.selection_mode,
            query: options.query.as_deref(),
            model: &options.model,
            estimated_tokens: options.estimated_tokens,
            redactions: options.redactions,
            git_commit: options.git_commit.as_deref(),
            budget_excluded: options.budget_excluded,
            skipped: options.skipped,
            omitted_count: options.omissions.len(),
            omissions: options
                .omissions
                .iter()
                .take(64)
                .map(|(path, reason)| JsonOmission {
                    path: rendered_path(path, options),
                    reason: reason.clone(),
                })
                .collect(),
        },
        files: json_files,
    };
    serde_json::to_string_pretty(&doc)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .map_err(|e| e.to_string())
}
