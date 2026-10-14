use directories::BaseDirs;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub default_minify: Option<bool>,
    #[serde(default)]
    pub global_ignore: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub max_output_mb: Option<usize>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub secret_policy: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub path_style: Option<String>,
    pub scan_max_entries: Option<usize>,
    pub scan_max_mb: Option<u64>,
    pub scan_max_depth: Option<usize>,
    pub scan_timeout: Option<u64>,
}

impl Config {
    pub fn for_args(args: &crate::cli::Args, cwd: &Path) -> Self {
        if args.mcp {
            // Agent requests use fixed server policy, never user/project overrides.
            Self::default()
        } else {
            Self::load(cwd)
        }
    }

    pub fn load(cwd: &Path) -> Self {
        let mut merged = Self::default();
        if let Some(global) = global_config_file() {
            if let Some(cfg) = load_file(&global) {
                merged.merge(cfg);
            }
        }

        let project = cwd.join(".cxt.toml");
        if project.is_file() {
            if let Some(cfg) = load_file(&project) {
                merged.merge(cfg);
            }
        }
        merged
    }

    fn merge(&mut self, other: Self) {
        if other.default_minify.is_some() {
            self.default_minify = other.default_minify;
        }
        if other.max_output_mb.is_some() {
            self.max_output_mb = other.max_output_mb;
        }
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        if other.model.is_some() {
            self.model = other.model;
        }
        if other.secret_policy.is_some() {
            self.secret_policy = other.secret_policy;
        }
        if other.format.is_some() {
            self.format = other.format;
        }
        if other.path_style.is_some() {
            self.path_style = other.path_style;
        }
        if other.scan_max_entries.is_some() {
            self.scan_max_entries = other.scan_max_entries;
        }
        if other.scan_max_mb.is_some() {
            self.scan_max_mb = other.scan_max_mb;
        }
        if other.scan_max_depth.is_some() {
            self.scan_max_depth = other.scan_max_depth;
        }
        if other.scan_timeout.is_some() {
            self.scan_timeout = other.scan_timeout;
        }
        self.global_ignore.extend(other.global_ignore);
        self.include.extend(other.include);
    }
}

fn load_file(path: &Path) -> Option<Config> {
    match fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => Some(config),
            Err(e) => {
                eprintln!("[cxt] warning: failed to parse {}: {}", path.display(), e);
                None
            }
        },
        Err(e) => {
            eprintln!("[cxt] warning: failed to read {}: {}", path.display(), e);
            None
        }
    }
}

fn global_config_file() -> Option<PathBuf> {
    let base_dirs = BaseDirs::new()?;
    let home = base_dirs.home_dir();
    let mut candidates = Vec::new();

    if let Some(project_dirs) = directories::ProjectDirs::from("", "", "cxt") {
        candidates.push(project_dirs.config_dir().join("config.toml"));
    }
    candidates.push(home.join(".config").join("cxt").join("config.toml"));
    candidates.push(home.join(".cxtrc"));
    candidates.into_iter().find(|p| p.is_file())
}

pub fn normalize_glob(pattern: &str) -> String {
    let trimmed = pattern.trim();
    if trimmed.starts_with('/') {
        trimmed.trim_start_matches('/').to_string()
    } else if trimmed.starts_with("**") || trimmed.contains('/') || trimmed.contains('\\') {
        trimmed.replace('\\', "/")
    } else {
        format!("**/{}", trimmed)
    }
}
