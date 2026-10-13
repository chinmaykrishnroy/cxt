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
}

#[derive(Serialize)]
struct JsonDocument<'a> {
    schema: &'static str,
    metadata: JsonMetadata<'a>,
    files: Vec<JsonFile<'a>>,
}

#[derive(Serialize)]
struct JsonMetadata<'a> {
    generated_at_unix: u64,
    selection_mode: &'a str,
    query: Option<&'a str>,
    model: &'a str,
    estimated_tokens: usize,
    redactions: usize,
    git_commit: Option<&'a str>,
}

#[derive(Serialize)]
struct JsonFile<'a> {
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
        OutputFormat::Xml => Ok(render_xml(files, options)),
        OutputFormat::Json => render_json(files, options),
    }
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
        out.push_str(&format!("- Selection: `{}`\n", options.selection_mode));
        if let Some(query) = &options.query {
            out.push_str(&format!("- Query: `{}`\n", query.replace('`', "\\`")));
        }
        out.push_str(&format!("- Model profile: `{}`\n", options.model));
        out.push_str(&format!(
            "- Estimated tokens: `{}`\n",
            options.estimated_tokens
        ));
        out.push_str(&format!("- Secret redactions: `{}`\n", options.redactions));
        if let Some(commit) = &options.git_commit {
            out.push_str(&format!("- Git commit: `{}`\n", commit));
        }
        out.push('\n');
    }

    for (index, file) in files.iter().enumerate() {
        if let Some(path) = rendered_path(&file.path, options) {
            out.push_str(&format!("### File: `{}`\n", path.replace('`', "\\`")));
        } else {
            out.push_str(&format!("### File {}\n", index + 1));
        }

        if options.manifest {
            out.push_str(&format!(
                "<!-- tokens={} score={} sha256={} reasons={} -->\n",
                file.tokens,
                file.score,
                sha256(&file.content),
                file.reasons.join(" | ").replace("--", "—")
            ));
        }

        let fence = if file.content.contains("````") {
            "`````"
        } else if file.content.contains("```") {
            "````"
        } else {
            "```"
        };
        out.push_str(&format!(
            "{}{}\n{}\n{}\n\n",
            fence, file.language, file.content, fence
        ));
    }

    out
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn render_xml(files: &[PackedFile], options: &RenderOptions) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<context>\n");
    out.push_str(&format!(
        "  <metadata selection=\"{}\" model=\"{}\" estimated_tokens=\"{}\" redactions=\"{}\"",
        xml_escape(&options.selection_mode),
        xml_escape(&options.model),
        options.estimated_tokens,
        options.redactions
    ));
    if let Some(commit) = &options.git_commit {
        out.push_str(&format!(" git_commit=\"{}\"", xml_escape(commit)));
    }
    out.push_str(" />\n");

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
            generated_at_unix: generated_at_unix(),
            selection_mode: &options.selection_mode,
            query: options.query.as_deref(),
            model: &options.model,
            estimated_tokens: options.estimated_tokens,
            redactions: options.redactions,
            git_commit: options.git_commit.as_deref(),
        },
        files: json_files,
    };
    serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())
}
