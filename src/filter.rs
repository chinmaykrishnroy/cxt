use crate::config::normalize_glob;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: u64 = 1024 * 1024;

pub struct FileFilter {
    includes: GlobSet,
    has_includes: bool,
    ignores: GlobSet,
    extensions: HashSet<String>,
    no_limit: bool,
    root: PathBuf,
}

impl FileFilter {
    pub fn new(
        root: PathBuf,
        includes: &[String],
        ignores: &[String],
        extensions: &[String],
        languages: &[String],
        no_limit: bool,
    ) -> Result<Self, String> {
        let includes_set = build_globset(includes)?;
        let ignores_set = build_globset(ignores)?;
        let mut exts: HashSet<String> = extensions
            .iter()
            .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
            .filter(|e| !e.is_empty())
            .collect();
        for language in languages {
            for ext in language_extensions(language) {
                exts.insert(ext.to_string());
            }
        }
        Ok(Self {
            includes: includes_set,
            has_includes: !includes.is_empty(),
            ignores: ignores_set,
            extensions: exts,
            no_limit,
            root,
        })
    }

    pub fn should_skip(&self, path: &Path) -> bool {
        if !path.is_file() {
            return true;
        }

        self.excludes_path(path)
            || (!self.no_limit && fs::metadata(path).is_ok_and(|m| m.len() > MAX_FILE_SIZE))
    }

    /// Path-only policy also applies to deleted files in a Git patch.
    pub fn excludes_path(&self, path: &Path) -> bool {
        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        let normalized = PathBuf::from(relative.to_string_lossy().replace('\\', "/"));
        let file_name = path.file_name().map(PathBuf::from).unwrap_or_default();

        if self.has_includes
            && !self.includes.is_match(&normalized)
            && !self.includes.is_match(&file_name)
        {
            return true;
        }
        if self.ignores.is_match(&normalized) || self.ignores.is_match(&file_name) {
            return true;
        }

        if !self.extensions.is_empty() {
            let ext = path
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !self.extensions.contains(&ext) {
                return true;
            }
        }

        if !self.no_limit && (is_junk_path(relative) || is_junk_file(path)) {
            return true;
        }
        false
    }
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let normalized = normalize_glob(pattern);
        let glob =
            Glob::new(&normalized).map_err(|e| format!("invalid glob '{}': {}", pattern, e))?;
        builder.add(glob);
    }
    builder.build().map_err(|e| e.to_string())
}

fn is_junk_path(path: &Path) -> bool {
    const DIR_NAMES: &[&str] = &[
        ".git",
        ".hg",
        ".svn",
        "target",
        "node_modules",
        "vendor",
        ".venv",
        "venv",
        "__pycache__",
        ".cache",
        ".next",
        ".nuxt",
        ".turbo",
        "coverage",
        "dist",
        "build",
        "out",
    ];
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        DIR_NAMES.iter().any(|junk| name.eq_ignore_ascii_case(junk))
    })
}

fn is_junk_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    name == ".env"
        || name.starts_with(".env.")
        || name == "credentials"
        || name == "credentials.json"
        || name == "service-account.json"
        || name == "service_account.json"
        || name == "id_rsa"
        || name == "id_ed25519"
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".p12")
        || name.ends_with(".pfx")
        || name.ends_with(".lock")
        || name.ends_with("-lock.json")
        || name == "package-lock.json"
        || name == "yarn.lock"
        || name == "pnpm-lock.yaml"
        || name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || matches!(
            ext.as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "webp"
                | "ico"
                | "svg"
                | "pdf"
                | "zip"
                | "gz"
                | "zst"
                | "7z"
                | "rar"
                | "woff"
                | "woff2"
                | "ttf"
                | "otf"
                | "mp3"
                | "mp4"
                | "mov"
                | "avi"
                | "mkv"
                | "bin"
                | "exe"
                | "dll"
                | "so"
                | "dylib"
        )
}

fn language_extensions(language: &str) -> &'static [&'static str] {
    match language.trim().to_ascii_lowercase().as_str() {
        "rust" => &["rs"],
        "python" | "py" => &["py"],
        "javascript" | "js" => &["js", "jsx", "mjs", "cjs"],
        "typescript" | "ts" => &["ts", "tsx", "mts", "cts"],
        "go" | "golang" => &["go"],
        "c" => &["c", "h"],
        "cpp" | "c++" => &["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
        "java" => &["java"],
        "csharp" | "c#" | "cs" => &["cs"],
        "ruby" | "rb" => &["rb"],
        "php" => &["php"],
        "json" => &["json"],
        "yaml" | "yml" => &["yaml", "yml"],
        "markdown" | "md" => &["md", "markdown"],
        _ => &[],
    }
}
