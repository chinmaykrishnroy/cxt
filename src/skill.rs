//! Portable, instruction-only agent integration. Exporting never scans source or edits agent configuration.
use std::io::Write;
use std::path::{Path, PathBuf};

pub const GUIDE: &str = include_str!("../skills/cxt/SKILL.md");

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillAgent {
    #[default]
    Generic,
    Codex,
    Claude,
    Antigravity,
    Devin,
    Kiro,
}

impl SkillAgent {
    pub fn default_path(self) -> &'static str {
        match self {
            Self::Generic => "cxt-skill/SKILL.md",
            Self::Codex | Self::Antigravity | Self::Devin => ".agents/skills/cxt/SKILL.md",
            Self::Claude => ".claude/skills/cxt/SKILL.md",
            Self::Kiro => ".kiro/skills/cxt/SKILL.md",
        }
    }
}

/// Write a complete guide atomically without replacing any existing destination.
pub fn export(
    cwd: &Path,
    destination: Option<&Path>,
    agent: SkillAgent,
) -> Result<PathBuf, String> {
    let path = cwd.join(destination.unwrap_or_else(|| Path::new(agent.default_path())));
    if path.file_name().is_none() || path.is_dir() {
        return Err(format!(
            "choose a SKILL.md file path, not a directory: {}",
            path.display()
        ));
    }
    let parent = path
        .parent()
        .ok_or("skill destination has no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("create skill directory: {e}"))?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("create skill file: {e}"))?;
    temporary
        .write_all(GUIDE.as_bytes())
        .map_err(|e| format!("write skill guide: {e}"))?;
    temporary.persist_noclobber(&path).map_err(|e| {
        format!(
            "cannot export skill to {} (existing files are never overwritten): {}",
            path.display(),
            e.error
        )
    })?;
    Ok(path)
}
