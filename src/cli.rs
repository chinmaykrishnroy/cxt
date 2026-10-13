use crate::output::{Compression, OutputFormat, PathStyle};
use crate::security::SecretPolicy;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[command(
    name = "cxt",
    author,
    version,
    about = "Fast, code-aware context engine for coding agents and LLMs",
    long_about = "Fast, code-aware context engine for coding agents and LLMs. Select, trace, rank, secure, budget, and render the smallest useful code context for an AI task."
)]
pub struct Args {
    /// Files/directories to scan. Defaults to the current directory.
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Read additional input paths from stdin, one path per line.
    #[arg(long)]
    pub stdin_paths: bool,

    /// Legacy alias for --git working.
    #[arg(short = 'd', long, conflicts_with = "git")]
    pub diff: bool,

    /// Git selection: working, staged, unstaged, untracked, tracked, or a commit/range such as HEAD~3..HEAD.
    #[arg(long = "git", value_name = "MODE_OR_RANGE")]
    pub git: Option<String>,

    /// Expand Git-selected files with reverse dependencies and likely related tests.
    #[arg(long)]
    pub impact: bool,

    /// AST-trace dependencies from file inputs. Multiple entry files are supported.
    #[arg(short = 't', long)]
    pub trace: bool,

    /// Rank repository files for a natural-language task, then expand relevant dependencies.
    #[arg(long, value_name = "TEXT")]
    pub query: Option<String>,

    /// Strip AST-recognized comments and blank lines.
    #[arg(short = 'm', long, conflicts_with = "no_minify")]
    pub minify: bool,

    /// Disable minification even when enabled by project/global configuration.
    #[arg(long, conflicts_with = "minify")]
    pub no_minify: bool,

    /// Include only paths matching this glob. Repeatable.
    #[arg(long = "include", value_name = "GLOB")]
    pub include: Vec<String>,

    /// Exclude paths matching this glob. Repeatable.
    #[arg(long = "ignore", value_name = "GLOB")]
    pub ignore: Vec<String>,

    /// Include only these extensions, comma-separated (for example rs,py,ts).
    #[arg(long = "ext", value_delimiter = ',', value_name = "EXT")]
    pub extensions: Vec<String>,

    /// Include only these language families, comma-separated (for example rust,python,typescript).
    #[arg(long = "lang", value_delimiter = ',', value_name = "LANG")]
    pub languages: Vec<String>,

    /// Exclude dotfiles and hidden directories. Hidden files remain included by default for backward compatibility.
    #[arg(long)]
    pub exclude_hidden: bool,

    /// Render output as markdown, xml, or json.
    #[arg(long, value_enum)]
    pub format: Option<OutputFormat>,

    /// How file paths appear in output headers.
    #[arg(long, value_enum)]
    pub path_style: Option<PathStyle>,

    /// Write the rendered context to a file.
    #[arg(short = 'o', long, value_name = "FILE", conflicts_with_all = ["stdout", "clipboard"])]
    pub output: Option<PathBuf>,

    /// Write only the rendered context to stdout; status messages go to stderr.
    #[arg(long, conflicts_with_all = ["output", "clipboard"])]
    pub stdout: bool,

    /// Force clipboard output (the default when no destination is selected).
    #[arg(long, conflicts_with_all = ["output", "stdout"])]
    pub clipboard: bool,

    /// Compress file output with gzip or zstd. Compression requires --output.
    #[arg(long, value_enum, default_value = "none")]
    pub compression: Compression,

    /// Token-accounting profile. Known values: heuristic, gpt-5, o200k, cl100k.
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    /// Estimated token budget including rendered overhead. 0 disables the budget.
    #[arg(long, alias = "budget", value_name = "TOKENS")]
    pub max_tokens: Option<usize>,

    /// Hard cap on candidate files after ranking/filtering. 0 disables the cap.
    #[arg(long, value_name = "N")]
    pub max_files: Option<usize>,

    /// Secret handling policy: redact, fail, or allow.
    #[arg(long, value_enum)]
    pub secrets: Option<SecretPolicy>,

    /// Add provenance metadata, selection reasons, hashes, and Git commit information.
    #[arg(long)]
    pub manifest: bool,

    /// Bypass per-file size and generated/junk safety limits. Binary files are still skipped.
    #[arg(long)]
    pub no_limit: bool,

    /// List selected files and reasons without rendering/copying context.
    #[arg(long)]
    pub dry_run: bool,

    /// CI-friendly mode: no clipboard fallback; stdout is used when --output is absent.
    #[arg(long, conflicts_with = "clipboard")]
    pub ci: bool,

    /// Open the interactive terminal UI for visual context selection.
    #[arg(short = 'T', long)]
    pub tui: bool,

    /// Generate shell completion script and exit.
    #[arg(long, value_enum, value_name = "SHELL")]
    #[serde(skip)]
    pub completions: Option<Shell>,
}

impl Args {
    pub fn normalized(mut self) -> Self {
        if self.paths.is_empty() && !self.stdin_paths {
            self.paths.push(PathBuf::from("."));
        }
        self
    }

    pub fn write_completions(shell: Shell) {
        let mut cmd = Self::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    }
}
