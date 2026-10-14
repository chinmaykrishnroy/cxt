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
    long_about = "Fast, code-aware context engine for coding agents and LLMs. Select, trace, rank, secure, budget, and render reviewable code context for an AI task. Selection is local; query ranking is lexical and structural, not semantic. Named cl100k/o200k encodings count packet tokens exactly; other profiles are estimates, not billing guarantees.",
    after_help = "Examples:\n  cxt                              Pack current directory to clipboard\n  cxt --tui                        Review interactively (-T; -t is trace)\n  cxt src/main.rs --trace --dry-run  Inspect entry file and resolved imports\n  cxt --git working --impact        Pack changed files, dependents and likely tests\n  cxt --query \"context budget\" --budget 20000 --output context.md\n  cxt --stdout                     Emit context only; status goes to stderr\n  cxt --scan-max-entries 50000 --scan-timeout 60 --dry-run\n\nSafety:\n  Home/ancestor/root scans require --allow-broad-scan. A scan-limit breach blocks export.\n  --no-limit does not disable scan caps; 0 disables an individual numeric cap.\n  Clipboard failure uses a new cxt-output file; existing fallback files are not overwritten.\n  Secret redaction is best-effort. Review before sharing; --secrets fail blocks findings.\n\nConfiguration: global config, then launch-directory .cxt.toml, then CLI scalar overrides.\nGuide and all options: https://github.com/chinmaykrishnroy/cxt/blob/main/docs/CLI_REFERENCE.md\nUse --help for expanded descriptions; --version prints the installed version."
)]
pub struct Args {
    /// Files/directories to scan. Defaults to the current directory.
    #[arg(value_name = "PATH", help_heading = "Input and selection")]
    pub paths: Vec<PathBuf>,

    /// Read additional input paths from stdin, one path per line.
    #[arg(long, help_heading = "Input and selection")]
    pub stdin_paths: bool,

    /// Legacy alias for --git working.
    #[arg(
        short = 'd',
        long,
        conflicts_with = "git",
        help_heading = "Input and selection"
    )]
    pub diff: bool,

    /// Git selection: working, staged, unstaged, untracked, tracked, or a commit/range such as HEAD~3..HEAD.
    #[arg(
        long = "git",
        value_name = "MODE_OR_RANGE",
        help_heading = "Input and selection"
    )]
    pub git: Option<String>,

    /// Require --git/--diff; add resolved reverse dependencies (up to two hops) and likely tests.
    #[arg(long, help_heading = "Input and selection")]
    pub impact: bool,

    /// AST-trace dependencies from file inputs. Multiple entry files are supported.
    #[arg(short = 't', long, help_heading = "Input and selection")]
    pub trace: bool,

    /// Rank task-relevant files/symbols and expand bounded identifier/import relationships (not semantic search).
    /// Focused/adaptive packets prioritize strong implementations before cheap supporting context.
    #[arg(long, value_name = "TEXT", help_heading = "Input and selection")]
    pub query: Option<String>,

    /// Explainable query ranker: hybrid, lexical BM25, symbols-only, or legacy substring scores for ablations.
    #[arg(
        long,
        value_enum,
        default_value = "hybrid",
        help_heading = "Input and selection"
    )]
    #[serde(default)]
    pub ranker: crate::query::Ranker,

    /// Disable automatic query dependency/reference expansion for controlled retrieval experiments.
    #[arg(long, help_heading = "Input and selection")]
    #[serde(default)]
    pub no_query_expansion: bool,

    /// Context representation: complete files, structural map, focused source spans, or adaptive fallback.
    #[arg(
        long = "context",
        value_enum,
        default_value = "complete",
        help_heading = "Transformation and budgets"
    )]
    #[serde(default)]
    pub context: crate::symbols::ContextLevel,

    /// Select a named or qualified AST symbol. Focused output retains original source spans and imports.
    #[arg(long, help_heading = "Input and selection")]
    #[serde(default)]
    pub symbol: Option<String>,

    /// Disable persistent symbol/dependency caching; source and resolver inputs are checked before reuse.
    #[arg(long, help_heading = "Safety")]
    #[serde(default)]
    pub no_cache: bool,

    /// Override the per-user metadata cache directory (symbols and dependency edges). No source bodies are cached.
    #[arg(long, help_heading = "Safety")]
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,

    /// Restrict source reads and dependency expansion to this canonical directory.
    #[arg(long, help_heading = "Safety")]
    #[serde(default)]
    pub workspace_root: Option<PathBuf>,

    /// Serve read-only MCP tools over stdio for one explicit workspace; no HTTP listener.
    #[arg(long, conflicts_with_all = ["tui", "stdin_paths", "output", "clipboard", "completions"], help_heading = "Review and scripting")]
    #[serde(default)]
    pub mcp: bool,

    /// Export the bundled agent SKILL.md without scanning. Optional file path; defaults to cxt-skill/SKILL.md or the selected agent's project location. Never overwrites.
    #[arg(long, num_args = 0..=1, value_name = "FILE", conflicts_with_all = ["paths", "tui", "mcp", "evaluate", "stdin_paths", "output", "stdout", "clipboard", "completions", "dry_run"], help_heading = "Agent integration")]
    #[serde(skip)]
    pub export_skill: Option<Option<PathBuf>>,

    /// Choose the skill export location convention. With --tui, sets the initial agent preference; never edits agent configuration.
    #[arg(long, value_enum, help_heading = "Agent integration")]
    #[serde(default)]
    pub skill_agent: Option<crate::skill::SkillAgent>,

    /// Run a versioned local retrieval corpus and emit a JSON evaluation report; executes this CLI in bounded child processes.
    #[arg(long, conflicts_with_all = ["mcp", "tui", "stdin_paths", "clipboard", "completions"], help_heading = "Review and scripting")]
    #[serde(default)]
    pub evaluate: Option<PathBuf>,

    /// Trusted legacy cxt executable for evaluation comparison; only used with --evaluate.
    #[arg(long, requires = "evaluate", help_heading = "Review and scripting")]
    #[serde(default)]
    pub eval_baseline: Option<PathBuf>,

    /// Compact parsed code conservatively: remove ordinary comments and safe blank lines, preserving literals and recognized directives. Unsupported/invalid syntax stays unchanged.
    #[arg(
        short = 'm',
        long,
        conflicts_with = "no_minify",
        help_heading = "Transformation and budgets"
    )]
    pub minify: bool,

    /// Disable minification even when enabled by project/global configuration.
    #[arg(
        long,
        conflicts_with = "minify",
        help_heading = "Transformation and budgets"
    )]
    pub no_minify: bool,

    /// Include only paths matching this glob. Repeatable.
    #[arg(long = "include", value_name = "GLOB", help_heading = "Filters")]
    pub include: Vec<String>,

    /// Exclude paths matching this glob. Repeatable.
    #[arg(long = "ignore", value_name = "GLOB", help_heading = "Filters")]
    pub ignore: Vec<String>,

    /// Include only these extensions, comma-separated (for example rs,py,ts).
    #[arg(
        long = "ext",
        value_delimiter = ',',
        value_name = "EXT",
        help_heading = "Filters"
    )]
    pub extensions: Vec<String>,

    /// Include only these language families, comma-separated (for example rust,python,typescript).
    #[arg(
        long = "lang",
        value_delimiter = ',',
        value_name = "LANG",
        help_heading = "Filters"
    )]
    pub languages: Vec<String>,

    /// Exclude dotfiles and hidden directories. Hidden files remain included by default for backward compatibility.
    #[arg(long, help_heading = "Filters")]
    pub exclude_hidden: bool,

    /// Output format: markdown (default unless configured), xml, or json.
    #[arg(long, value_enum, help_heading = "Output")]
    pub format: Option<OutputFormat>,

    /// Output header paths: relative (default unless configured), absolute, or none.
    #[arg(long, value_enum, help_heading = "Output")]
    pub path_style: Option<PathStyle>,

    /// Write context to a file; an existing named output can be overwritten.
    #[arg(short = 'o', long, value_name = "FILE", conflicts_with_all = ["stdout", "clipboard"], help_heading = "Output")]
    pub output: Option<PathBuf>,

    /// Write only the rendered context to stdout; status messages go to stderr.
    #[arg(long, conflicts_with_all = ["output", "clipboard"], help_heading = "Output")]
    pub stdout: bool,

    /// Force clipboard output (the default when no destination is selected).
    #[arg(long, conflicts_with_all = ["output", "stdout"], help_heading = "Output")]
    pub clipboard: bool,

    /// Compress file output with gzip or zstd. Compression requires --output.
    #[arg(long, value_enum, default_value = "none", help_heading = "Output")]
    pub compression: Compression,

    /// Offline token accounting: cl100k/o200k are exact named encodings; heuristic/code/gpt-5 are estimates. Packet tokens are not chat/billing guarantees.
    #[arg(
        long,
        value_name = "MODEL",
        help_heading = "Transformation and budgets"
    )]
    pub model: Option<String>,

    /// Token budget including rendered overhead, exact for named encodings and estimated otherwise. 0 disables the budget.
    #[arg(
        long,
        visible_alias = "budget",
        value_name = "TOKENS",
        help_heading = "Transformation and budgets"
    )]
    pub max_tokens: Option<usize>,

    /// Cap candidates after ranking/filtering (query default 40; otherwise unlimited). 0 disables it.
    #[arg(long, value_name = "N", help_heading = "Transformation and budgets")]
    pub max_files: Option<usize>,

    /// Best-effort secret policy: redact (default unless configured), fail, or allow. Review before sharing.
    #[arg(long, value_enum, help_heading = "Safety")]
    pub secrets: Option<SecretPolicy>,

    /// Secret scanning backend: builtin patterns, or builtin plus installed Gitleaks. Scanner failures block export.
    #[arg(long, value_enum, default_value = "builtin", help_heading = "Safety")]
    #[serde(default)]
    pub secret_scanner: crate::security::SecretScanner,

    /// Explicit regex allowlist for known example findings. Repeatable; broad rules can bypass protection.
    #[arg(long, help_heading = "Safety")]
    #[serde(default)]
    pub secret_allow: Vec<String>,

    /// Add provenance metadata, selection reasons, hashes, and Git commit information.
    #[arg(long, help_heading = "Output")]
    pub manifest: bool,

    /// Bypass per-file size/junk rules, not scan caps or broad-folder guards. Binary files are skipped.
    #[arg(long, help_heading = "Safety")]
    pub no_limit: bool,

    /// Explicitly permit a home-directory, ancestor, or filesystem-root scan. Scan limits still apply.
    #[arg(long, help_heading = "Safety")]
    #[serde(default)]
    pub allow_broad_scan: bool,

    /// Maximum visited files/directories per scan phase (default 20000). 0 disables this limit.
    #[arg(long, value_name = "N", help_heading = "Safety")]
    #[serde(default)]
    pub scan_max_entries: Option<usize>,

    /// Maximum eligible input MiB per scan phase (default 100). 0 disables this limit.
    #[arg(long, value_name = "MB", help_heading = "Safety")]
    #[serde(default)]
    pub scan_max_mb: Option<u64>,

    /// Maximum traversal depth (default 32). 0 disables this limit.
    #[arg(long, value_name = "N", help_heading = "Safety")]
    #[serde(default)]
    pub scan_max_depth: Option<usize>,

    /// Cooperative scan time limit per phase in seconds (default 15). 0 disables this limit.
    #[arg(long, value_name = "SECONDS", help_heading = "Safety")]
    #[serde(default)]
    pub scan_timeout: Option<u64>,

    /// List final files/reasons without copying or writing context. Still scans and transforms inputs.
    #[arg(long, help_heading = "Review and scripting")]
    pub dry_run: bool,

    /// CI-friendly mode: no clipboard fallback; stdout is used when --output is absent.
    #[arg(
        long,
        conflicts_with = "clipboard",
        help_heading = "Review and scripting"
    )]
    pub ci: bool,

    /// Open interactive review in a terminal. Bare cxt stays CLI-first; -t means trace.
    #[arg(short = 'T', long, help_heading = "Review and scripting")]
    pub tui: bool,

    /// Generate shell completion script and exit.
    #[arg(
        long,
        value_enum,
        value_name = "SHELL",
        help_heading = "Review and scripting"
    )]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cli_option_is_in_both_help_forms_and_the_user_reference() {
        let mut command = Args::command();
        let short = command.render_help().to_string();
        let long = command.render_long_help().to_string();
        let reference = include_str!("../docs/CLI_REFERENCE.md");
        assert_eq!(
            reference.lines().next(),
            Some("# cxt user guide and CLI reference"),
            "the rolling reference must not pin a stale release version"
        );
        for argument in command.get_arguments() {
            if let Some(name) = argument.get_long() {
                let flag = format!("--{name}");
                assert!(short.contains(&flag), "missing short-help option {flag}");
                assert!(long.contains(&flag), "missing long-help option {flag}");
                assert!(reference.contains(&flag), "missing reference option {flag}");
            }
            if let Some(name) = argument.get_short() {
                let flag = format!("-{name}");
                assert!(
                    reference.contains(&flag),
                    "missing reference short option {flag}"
                );
            }
        }
        for help in [short, long] {
            for required in [
                "Examples:",
                "Safety:",
                "--budget",
                "Input and selection:",
                "--scan-timeout",
                "Configuration:",
                "--allow-broad-scan",
            ] {
                assert!(help.contains(required), "missing help guidance {required}");
            }
        }
    }

    #[test]
    fn documented_budget_alias_and_trace_tui_flags_parse_as_expected() {
        let args = Args::parse_from(["cxt", "src/main.rs", "-t", "-T", "--budget", "20000"]);
        assert!(args.trace);
        assert!(args.tui);
        assert_eq!(args.max_tokens, Some(20000));
        assert_eq!(args.paths, vec![PathBuf::from("src/main.rs")]);
    }
}
