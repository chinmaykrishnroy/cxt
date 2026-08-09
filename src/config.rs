use directories::BaseDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    /// Whether to enable minification by default when --minify is not passed.
    #[serde(default)]
    pub default_minify: bool,

    /// Glob patterns for files to always ignore during directory scans.
    #[serde(default)]
    pub global_ignore: Vec<String>,

    /// Optional output limit override in megabytes. Defaults to 100MB in main.
    #[serde(default)]
    pub max_output_mb: Option<usize>,
}

impl Config {
    pub fn load() -> Self {
        if let Some(path) = find_config_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                return toml::from_str(&content).unwrap_or_else(|e| {
                    eprintln!(
                        "[cxt] warning: failed to parse config at {}: {}",
                        path.display(),
                        e
                    );
                    Self::default()
                });
            }
        }
        Self::default()
    }
}

fn find_config_file() -> Option<PathBuf> {
    let base_dirs = BaseDirs::new()?;
    let home = base_dirs.home_dir();

    let mut candidates = Vec::new();

    // Primary: ProjectDirs config directory (e.g. ~/.config/cxt/config.toml or AppData\Roaming\cxt\config.toml)
    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "cxt") {
        candidates.push(proj_dirs.config_dir().join("config.toml"));
    }

    // Fallback: ~/.config/cxt/config.toml and the legacy plain home config file
    candidates.push(home.join(".config").join("cxt").join("config.toml"));
    candidates.push(home.join(".cxtrc"));

    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// Normalizes a user-provided ignore glob so `*.csv` matches files in any directory.
pub fn normalize_ignore_pattern(pattern: &str) -> String {
    let trimmed = pattern.trim();
    if trimmed.starts_with('/') {
        // Root-anchored pattern: strip leading slash but do not prefix with **.
        trimmed.trim_start_matches('/').to_string()
    } else if trimmed.starts_with("**") || trimmed.contains('/') {
        trimmed.to_string()
    } else {
        format!("**/{}", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ignore_pattern() {
        assert_eq!(normalize_ignore_pattern("*.csv"), "**/*.csv");
        assert_eq!(normalize_ignore_pattern("**/*.json"), "**/*.json");
        assert_eq!(normalize_ignore_pattern("/foo.tmp"), "foo.tmp");
        assert_eq!(normalize_ignore_pattern("dir/*.log"), "dir/*.log");
    }
}
