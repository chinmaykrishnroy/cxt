use crate::cli::Args;
use crate::engine::EngineResult;
use crate::output::{Compression, OutputFormat, PathStyle};
use crate::review::{Plan, TaskSession, Workspace};
use crate::security::SecretPolicy;
use clap::ValueEnum;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub struct TuiSelection {
    pub args: Args,
    pub result: EngineResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Source,
    Changes,
    Dependencies,
    Checks,
    Output,
    Settings,
    Symbols,
}
const PANELS: &[Panel] = &[
    Panel::Source,
    Panel::Changes,
    Panel::Dependencies,
    Panel::Checks,
    Panel::Output,
    Panel::Settings,
    Panel::Symbols,
];
impl Panel {
    fn title(self) -> &'static str {
        match self {
            Self::Source => "1 Source",
            Self::Changes => "2 Changes",
            Self::Dependencies => "3 Dependencies",
            Self::Checks => "4 Checks",
            Self::Output => "5 Output",
            Self::Settings => "6 Settings",
            Self::Symbols => "7 Symbols",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Setting {
    Paths,
    Git,
    Query,
    Ranker,
    NoQueryExpansion,
    Budget,
    Model,
    Context,
    Symbol,
    NoCache,
    CacheDir,
    Scanner,
    SecretAllow,
    WorkspaceRoot,
    SkillAgent,
    Format,
    PathStyle,
    Secrets,
    Minify,
    Trace,
    Impact,
    Manifest,
    Include,
    Ignore,
    Extensions,
    Languages,
    Hidden,
    MaxFiles,
    NoLimit,
    AllowBroad,
    ScanEntries,
    ScanMb,
    ScanDepth,
    ScanTimeout,
    Destination,
    OutputPath,
    Compression,
    DryRun,
    Ci,
    Wrap,
}
const SETTINGS: &[Setting] = &[
    Setting::Paths,
    Setting::Git,
    Setting::Query,
    Setting::Ranker,
    Setting::NoQueryExpansion,
    Setting::Budget,
    Setting::Model,
    Setting::Context,
    Setting::Symbol,
    Setting::NoCache,
    Setting::CacheDir,
    Setting::Scanner,
    Setting::SecretAllow,
    Setting::WorkspaceRoot,
    Setting::SkillAgent,
    Setting::Format,
    Setting::PathStyle,
    Setting::Secrets,
    Setting::Minify,
    Setting::Trace,
    Setting::Impact,
    Setting::Manifest,
    Setting::Include,
    Setting::Ignore,
    Setting::Extensions,
    Setting::Languages,
    Setting::Hidden,
    Setting::MaxFiles,
    Setting::NoLimit,
    Setting::AllowBroad,
    Setting::ScanEntries,
    Setting::ScanMb,
    Setting::ScanDepth,
    Setting::ScanTimeout,
    Setting::Destination,
    Setting::OutputPath,
    Setting::Compression,
    Setting::DryRun,
    Setting::Ci,
    Setting::Wrap,
];
impl Setting {
    fn choices(self) -> &'static [&'static str] {
        match self {
            Self::Git => &[
                "all",
                "working",
                "staged",
                "unstaged",
                "untracked",
                "tracked",
            ],
            Self::Model => &["heuristic", "code", "gpt-5", "o200k", "cl100k"],
            Self::Context => &["complete", "map", "focused", "adaptive"],
            Self::Ranker => &["hybrid", "lexical", "symbols", "legacy"],
            Self::Scanner => &["builtin", "gitleaks"],
            Self::SkillAgent => &["generic", "codex", "claude", "antigravity", "devin", "kiro"],
            Self::Format => &["markdown", "xml", "json"],
            Self::PathStyle => &["relative", "absolute", "none"],
            Self::Secrets => &["redact", "fail", "allow"],
            Self::Destination => &["clipboard", "stdout", "file"],
            Self::Compression => &["none", "gzip", "zstd"],
            Self::Minify
            | Self::NoCache
            | Self::NoQueryExpansion
            | Self::Trace
            | Self::Impact
            | Self::Manifest
            | Self::Hidden
            | Self::NoLimit
            | Self::AllowBroad
            | Self::DryRun
            | Self::Ci
            | Self::Wrap => &["false", "true"],
            _ => &[],
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Paths => "Input paths (; separated)",
            Self::Git => "Git selection",
            Self::Query => "Task query",
            Self::Ranker => "Query ranking strategy",
            Self::NoQueryExpansion => "Disable query expansion",
            Self::Budget => "Token budget",
            Self::Model => "Token profile",
            Self::Context => "Context representation",
            Self::Symbol => "AST symbol name (y)",
            Self::NoCache => "Disable metadata cache",
            Self::CacheDir => "Metadata cache directory",
            Self::Scanner => "Secret scanner backend",
            Self::SecretAllow => "Secret allowlist regexes (; separated)",
            Self::WorkspaceRoot => "Restrict workspace root",
            Self::SkillAgent => "Skill export agent (E)",
            Self::Format => "Output format",
            Self::PathStyle => "Path style",
            Self::Secrets => "Secret policy",
            Self::Minify => "Minify",
            Self::Trace => "Trace imports",
            Self::Impact => "Change impact",
            Self::Manifest => "Provenance manifest",
            Self::Include => "Include globs (; separated)",
            Self::Ignore => "Ignore globs (; separated)",
            Self::Extensions => "Extensions (comma separated)",
            Self::Languages => "Languages (comma separated)",
            Self::Hidden => "Exclude hidden files",
            Self::MaxFiles => "Maximum candidate files",
            Self::NoLimit => "Bypass size/junk limits",
            Self::AllowBroad => "Allow broad scan",
            Self::ScanEntries => "Scan entry limit",
            Self::ScanMb => "Scan input MiB limit",
            Self::ScanDepth => "Scan depth limit",
            Self::ScanTimeout => "Scan time limit (seconds)",
            Self::Destination => "Destination",
            Self::OutputPath => "Output filename",
            Self::Compression => "Compression",
            Self::DryRun => "Dry run",
            Self::Ci => "CI output",
            Self::Wrap => "Wrap preview",
        }
    }
    fn hint(self) -> &'static str {
        match self {
        Self::Git => "all, working, staged, unstaged, untracked, tracked, or a comparison such as main..HEAD",
        Self::Budget | Self::MaxFiles => "Positive integer; 0 or empty means unlimited. cl100k/o200k count exact packet encoding tokens; other profiles estimate.",
        Self::Format => "markdown, xml, json", Self::PathStyle => "relative, absolute, none",
        Self::Secrets => "redact, fail, allow. Fail sanitizes the preview and blocks export when findings exist.",
        Self::Model => "cl100k/o200k count exact encoding tokens offline. heuristic/code/gpt-5 remain estimates, not billing guarantees.",
        Self::Context => "complete: full files; map: signatures only; focused: selected AST spans/imports; adaptive: map fallback when implementation does not fit. Not proof of semantic completeness.",
        Self::Ranker | Self::NoQueryExpansion => "Ablation controls: hybrid lexical/symbol/reference signals, lexical BM25, symbols-only, or legacy substring weights. Expansion is approximate, not a resolved call graph.",
        Self::Symbol => "Named or qualified symbol; empty clears it. 7 shows symbols in the selected file. y opens this selector.",
        Self::NoCache | Self::CacheDir => "Validated symbol/dependency metadata; source bodies are not persisted. Source and resolver inputs are rechecked. Empty directory uses the per-user cache.",
        Self::Scanner => "builtin, or builtin plus separately installed Gitleaks. Scanner failures block export.",
        Self::SecretAllow => "Explicit regex exceptions for known examples; broad allowlists can leak secrets. Separate rules with semicolons.",
        Self::WorkspaceRoot => "Empty disables the explicit root restriction. Canonical source/dependency paths must remain within this directory.",
        Self::SkillAgent => "Project-local skill location convention. E exports a static guide to a new file; no agent configuration is changed and no source is included.",
        Self::Destination => "clipboard, stdout, file. File requires an output filename.",
        Self::Compression => "none, gzip, zstd. Compression requires a file destination.",
        Self::OutputPath => "Filename relative to the launch directory; existing output files may be overwritten.",
        Self::Paths | Self::Include | Self::Ignore => "Separate entries with semicolons. Selection settings refresh in the background.",
        Self::Query => "Local keyword ranking; empty clears the query. No network or LLM requests.",
        Self::Trace => "Resolve local static imports. Dynamic imports and unsupported layouts may be incomplete.",
        Self::Impact => "Requires Git selection; suggests reverse imports and likely related tests.",
        Self::NoLimit => "true bypasses file-size/junk limits; binaries remain excluded.",
        Self::AllowBroad => "WARNING: true permits scanning your home directory, its ancestors or filesystem root. Narrow to a project instead when possible. Scan limits still apply.",
        Self::ScanEntries | Self::ScanMb | Self::ScanDepth | Self::ScanTimeout => "Positive integer; 0 disables this specific limit. Time limits are cooperative, not a hard OS timeout.",
        Self::Extensions | Self::Languages => "Comma-separated values; empty includes all values.",
        _ => "true or false",
    }
    }
}

#[derive(Debug, Clone)]
enum Row {
    Directory(PathBuf),
    File(usize),
    Change(usize),
}
#[derive(Debug, Clone, Copy)]
enum Edit {
    Setting(Setting),
    Search,
    Save,
    Load,
    Locate,
    Skill,
}
struct Input {
    edit: Edit,
    text: String,
    cursor: usize,
}
impl Input {
    fn insert(&mut self, text: &str) {
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }
    fn left(&mut self) {
        self.cursor = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }
    fn right(&mut self) {
        self.cursor += self.text[self.cursor..]
            .chars()
            .next()
            .map_or(0, char::len_utf8);
    }
    fn choose(&mut self, delta: isize) {
        let Edit::Setting(setting) = self.edit else {
            return;
        };
        let choices = setting.choices();
        if choices.is_empty() {
            return;
        }
        let current = choices.iter().position(|s| *s == self.text).unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(choices.len() as isize) as usize;
        self.text = choices[next].into();
        self.cursor = self.text.len();
    }
}
enum Work {
    Load(Args),
    Plan(Arc<Workspace>, Args, BTreeSet<PathBuf>, BTreeSet<PathBuf>),
    Export(
        Arc<Workspace>,
        Args,
        BTreeSet<PathBuf>,
        BTreeSet<PathBuf>,
        EngineResult,
    ),
}
struct Request {
    revision: u64,
    work: Work,
}
struct Response {
    revision: u64,
    workspace: Option<Arc<Workspace>>,
    plan: Option<Plan>,
    export: Option<TuiSelection>,
    error: Option<String>,
}

fn worker(cwd: PathBuf) -> (mpsc::Sender<Request>, mpsc::Receiver<Response>) {
    let (tx, rx) = mpsc::channel::<Request>();
    let (done, responses) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(mut request) = rx.recv() {
            for newer in rx.try_iter() {
                request = newer;
            }
            let mut response = Response {
                revision: request.revision,
                workspace: None,
                plan: None,
                export: None,
                error: None,
            };
            match request.work {
                Work::Load(args) => match Workspace::load(&args, &cwd) {
                    Ok(workspace) => response.workspace = Some(Arc::new(workspace)),
                    Err(error) => response.error = Some(error),
                },
                Work::Plan(workspace, args, selected, pinned) => {
                    response.plan = Some(workspace.plan(&args, &cwd, &selected, &pinned))
                }
                Work::Export(workspace, args, selected, pinned, result) => {
                    let stale = workspace.stale_paths(&selected);
                    if !stale.is_empty() || !workspace.git_is_current() {
                        response.error = Some(format!("{} source file(s) changed since preview. Press r to refresh before exporting.", stale.len()));
                    } else {
                        if pinned
                            .iter()
                            .all(|p| result.packed_files.iter().any(|f| &f.path == p))
                        {
                            response.export = Some(TuiSelection { args, result });
                        } else {
                            response.error = Some(
                                "Pinned files do not fit; increase budget or unpin them.".into(),
                            );
                        }
                    }
                }
            }
            if done.send(response).is_err() {
                break;
            }
        }
    });
    (tx, responses)
}

struct App {
    args: Args,
    cwd: PathBuf,
    workspace: Option<Arc<Workspace>>,
    plan: Option<Plan>,
    selected: BTreeSet<PathBuf>,
    pinned: BTreeSet<PathBuf>,
    collapsed: HashSet<PathBuf>,
    panel: Panel,
    cursor: usize,
    list: ListState,
    settings_cursor: usize,
    settings_list: ListState,
    filter: String,
    scroll: (u16, u16),
    preview_focus: bool,
    wrap: bool,
    input: Option<Input>,
    help: bool,
    status: String,
    revision: u64,
    busy: bool,
    tick: usize,
    tx: mpsc::Sender<Request>,
    rx: mpsc::Receiver<Response>,
    pending_session: Option<TaskSession>,
    tree_area: Rect,
    preview_area: Rect,
    settings_area: Rect,
    tab_hits: Vec<(Panel, Rect)>,
}
impl App {
    fn new(args: Args, cwd: &Path) -> Self {
        let (tx, rx) = worker(cwd.to_path_buf());
        let mut app = Self {
            args,
            cwd: cwd.to_path_buf(),
            workspace: None,
            plan: None,
            selected: BTreeSet::new(),
            pinned: BTreeSet::new(),
            collapsed: HashSet::new(),
            panel: Panel::Source,
            cursor: 0,
            list: ListState::default(),
            settings_cursor: 0,
            settings_list: ListState::default(),
            filter: String::new(),
            scroll: (0, 0),
            preview_focus: false,
            wrap: true,
            input: None,
            help: false,
            status: "Preparing workspace…".into(),
            revision: 0,
            busy: false,
            tick: 0,
            tx,
            rx,
            pending_session: None,
            tree_area: Rect::default(),
            preview_area: Rect::default(),
            settings_area: Rect::default(),
            tab_hits: Vec::new(),
        };
        app.args.tui = false;
        let config = crate::config::Config::load(cwd);
        app.args.max_tokens = app.args.max_tokens.or(config.max_tokens);
        app.args.model = app.args.model.or(config.model);
        app.args.format = app.args.format.or(config
            .format
            .as_deref()
            .and_then(|s| OutputFormat::from_str(s, true).ok()));
        app.args.path_style = app.args.path_style.or(config
            .path_style
            .as_deref()
            .and_then(|s| PathStyle::from_str(s, true).ok()));
        app.args.secrets = app.args.secrets.or(config
            .secret_policy
            .as_deref()
            .and_then(|s| SecretPolicy::from_str(s, true).ok()));
        app.args.minify =
            !app.args.no_minify && (app.args.minify || config.default_minify.unwrap_or(false));
        app.args.no_minify = !app.args.minify;
        let limits = crate::scan::ScanLimits::for_args(&app.args, cwd);
        app.args.scan_max_entries = Some(limits.max_entries);
        app.args.scan_max_mb = Some(limits.max_bytes / (1024 * 1024));
        app.args.scan_max_depth = Some(limits.max_depth);
        app.args.scan_timeout = Some(limits.timeout.as_secs());
        app.load();
        app
    }
    fn submit(&mut self, work: Work) {
        self.revision += 1;
        self.busy = true;
        if self
            .tx
            .send(Request {
                revision: self.revision,
                work,
            })
            .is_err()
        {
            self.busy = false;
            self.status = "Background worker stopped".into();
        }
    }
    fn load(&mut self) {
        self.plan = None;
        self.submit(Work::Load(self.args.clone()));
    }
    fn replan(&mut self) {
        self.plan = None;
        if let Some(workspace) = self.workspace.clone() {
            self.submit(Work::Plan(
                workspace,
                self.args.clone(),
                self.selected.clone(),
                self.pinned.clone(),
            ));
        }
    }
    fn poll(&mut self) -> Option<TuiSelection> {
        loop {
            let response = match self.rx.try_recv() {
                Ok(response) => response,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.busy = false;
                    self.plan = None;
                    self.workspace = None;
                    self.pending_session = None;
                    self.status = "Background worker stopped; quit and restart cxt".into();
                    break;
                }
            };
            if response.revision != self.revision {
                continue;
            }
            self.busy = false;
            if let Some(error) = response.error {
                self.pending_session = None;
                if self.plan.is_none() {
                    self.workspace = None;
                }
                self.status = error;
            }
            if let Some(workspace) = response.workspace {
                let available: BTreeSet<_> = workspace
                    .files
                    .iter()
                    .map(|s| s.file.path.clone())
                    .collect();
                if let Some(session) = self.pending_session.take() {
                    let changed = session
                        .fingerprints
                        .iter()
                        .filter(|(path, hash)| {
                            workspace
                                .files
                                .iter()
                                .find(|f| &f.file.path == *path)
                                .is_none_or(|f| &f.source_hash != *hash)
                        })
                        .count();
                    self.selected = session
                        .selected
                        .into_iter()
                        .filter(|p| available.contains(p))
                        .collect();
                    self.pinned = session.pinned.into_iter().collect();
                    self.status = format!(
                        "Task restored; {} saved file(s) changed or unavailable",
                        changed
                    );
                } else if let Some(previous_workspace) = &self.workspace {
                    let previous: BTreeSet<_> = previous_workspace
                        .files
                        .iter()
                        .map(|s| s.file.path.clone())
                        .collect();
                    self.selected.retain(|p| available.contains(p));
                    self.selected
                        .extend(available.difference(&previous).cloned());
                    self.status =
                        "Workspace refreshed; pins remain until explicitly cleared".into();
                } else {
                    self.selected = available;
                    self.status =
                        "Ready. Choose a workflow tab; g exports the reviewed plan.".into();
                }
                self.workspace = Some(workspace);
                self.cursor = 0;
                self.scroll = (0, 0);
                self.replan();
            }
            if let Some(plan) = response.plan {
                self.plan = Some(plan);
            }
            if let Some(export) = response.export {
                return Some(export);
            }
        }
        None
    }
    fn rows(&self) -> Vec<Row> {
        let Some(workspace) = &self.workspace else {
            return Vec::new();
        };
        if self.panel == Panel::Changes {
            return workspace
                .changes
                .iter()
                .enumerate()
                .filter(|(_, c)| self.matches(&workspace.relative(&c.path)))
                .map(|(i, _)| Row::Change(i))
                .collect();
        }
        let mut rows = Vec::new();
        let mut dirs = BTreeSet::new();
        let mut files: Vec<_> = workspace.files.iter().enumerate().collect();
        files.sort_by_key(|(_, f)| &f.file.path);
        for (i, snapshot) in files {
            let relative = snapshot
                .file
                .path
                .strip_prefix(&workspace.prepared.root)
                .unwrap_or(&snapshot.file.path);
            if !self.matches(&relative.display().to_string()) {
                continue;
            }
            let mut dir = workspace.prepared.root.clone();
            let parts: Vec<_> = relative.components().collect();
            let mut hidden = false;
            for part in parts.iter().take(parts.len().saturating_sub(1)) {
                dir.push(part.as_os_str());
                if dirs.insert(dir.clone()) {
                    rows.push(Row::Directory(dir.clone()));
                }
                if self.filter.is_empty() && self.collapsed.contains(&dir) {
                    hidden = true;
                    break;
                }
            }
            if !hidden {
                rows.push(Row::File(i));
            }
        }
        rows
    }
    fn matches(&self, path: &str) -> bool {
        let lower = path.to_lowercase();
        self.filter
            .to_lowercase()
            .split_whitespace()
            .all(|term| lower.contains(term))
    }
    fn current_path(&self) -> Option<PathBuf> {
        let workspace = self.workspace.as_ref()?;
        match self.rows().get(self.cursor)? {
            Row::Directory(p) => Some(p.clone()),
            Row::File(i) => Some(workspace.files[*i].file.path.clone()),
            Row::Change(i) => Some(workspace.changes[*i].path.clone()),
        }
    }
    fn move_cursor(&mut self, amount: isize) {
        if self.panel == Panel::Settings {
            self.settings_cursor = self
                .settings_cursor
                .saturating_add_signed(amount)
                .min(SETTINGS.len() - 1);
        } else {
            self.cursor = self
                .cursor
                .saturating_add_signed(amount)
                .min(self.rows().len().saturating_sub(1));
        }
        self.scroll = (0, 0);
    }
    fn switch(&mut self, panel: Panel) {
        self.panel = panel;
        self.cursor = 0;
        self.scroll = (0, 0);
        self.preview_focus = false;
    }
    fn toggle(&mut self) {
        if self.busy {
            self.status = "Wait for refresh before changing selection".into();
            return;
        }
        let Some(path) = self.current_path() else {
            return;
        };
        let Some(workspace) = &self.workspace else {
            return;
        };
        let paths: Vec<_> = workspace
            .files
            .iter()
            .filter(|f| f.file.path == path || f.file.path.starts_with(&path))
            .map(|f| f.file.path.clone())
            .collect();
        let all = paths.iter().all(|p| self.selected.contains(p));
        for path in paths {
            if all {
                self.selected.remove(&path);
                self.pinned.remove(&path);
            } else {
                self.selected.insert(path);
            }
        }
        self.replan();
    }
    fn pin(&mut self) {
        if self.busy {
            return;
        }
        let Some(path) = self.current_path() else {
            return;
        };
        if !self
            .workspace
            .as_ref()
            .is_some_and(|w| w.files.iter().any(|f| f.file.path == path))
        {
            return;
        }
        if !self.pinned.remove(&path) {
            self.pinned.insert(path.clone());
            self.selected.insert(path);
        }
        self.replan();
    }
    fn fold(&mut self) {
        if let Some(Row::Directory(path)) = self.rows().get(self.cursor) {
            if !self.collapsed.remove(path) {
                self.collapsed.insert(path.clone());
            }
        }
    }
    fn setting_value(&self, setting: Setting) -> String {
        let a = &self.args;
        match setting {
            Setting::Paths => a
                .paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("; "),
            Setting::Git => a
                .git
                .clone()
                .unwrap_or_else(|| if a.diff { "working" } else { "all" }.into()),
            Setting::Query => a.query.clone().unwrap_or_default(),
            Setting::Ranker => format!("{:?}", a.ranker).to_lowercase(),
            Setting::NoQueryExpansion => a.no_query_expansion.to_string(),
            Setting::Budget => a.max_tokens.unwrap_or(0).to_string(),
            Setting::Model => a.model.clone().unwrap_or_else(|| "heuristic".into()),
            Setting::Context => format!("{:?}", a.context).to_lowercase(),
            Setting::Symbol => a.symbol.clone().unwrap_or_default(),
            Setting::NoCache => a.no_cache.to_string(),
            Setting::CacheDir => a
                .cache_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            Setting::Scanner => format!("{:?}", a.secret_scanner).to_lowercase(),
            Setting::SecretAllow => a.secret_allow.join("; "),
            Setting::WorkspaceRoot => a
                .workspace_root
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            Setting::Format => format!("{:?}", a.format.unwrap_or_default()).to_lowercase(),
            Setting::SkillAgent => {
                format!("{:?}", a.skill_agent.unwrap_or_default()).to_lowercase()
            }
            Setting::PathStyle => format!("{:?}", a.path_style.unwrap_or_default()).to_lowercase(),
            Setting::Secrets => format!("{:?}", a.secrets.unwrap_or_default()).to_lowercase(),
            Setting::Minify => a.minify.to_string(),
            Setting::Trace => a.trace.to_string(),
            Setting::Impact => a.impact.to_string(),
            Setting::Manifest => a.manifest.to_string(),
            Setting::Include => a.include.join("; "),
            Setting::Ignore => a.ignore.join("; "),
            Setting::Extensions => a.extensions.join(","),
            Setting::Languages => a.languages.join(","),
            Setting::Hidden => a.exclude_hidden.to_string(),
            Setting::MaxFiles => a
                .max_files
                .unwrap_or(if a.query.is_some() { 40 } else { 0 })
                .to_string(),
            Setting::NoLimit => a.no_limit.to_string(),
            Setting::AllowBroad => a.allow_broad_scan.to_string(),
            Setting::ScanEntries => a.scan_max_entries.unwrap_or(20_000).to_string(),
            Setting::ScanMb => a.scan_max_mb.unwrap_or(100).to_string(),
            Setting::ScanDepth => a.scan_max_depth.unwrap_or(32).to_string(),
            Setting::ScanTimeout => a.scan_timeout.unwrap_or(15).to_string(),
            Setting::Destination => if a.output.is_some() {
                "file"
            } else if a.stdout || a.ci {
                "stdout"
            } else {
                "clipboard"
            }
            .into(),
            Setting::OutputPath => a
                .output
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            Setting::Compression => format!("{:?}", a.compression).to_lowercase(),
            Setting::DryRun => a.dry_run.to_string(),
            Setting::Ci => a.ci.to_string(),
            Setting::Wrap => self.wrap.to_string(),
        }
    }
    fn edit(&mut self, edit: Edit) {
        let text = match edit {
            Edit::Setting(s) => self.setting_value(s),
            Edit::Search => self.filter.clone(),
            Edit::Save | Edit::Load => ".cxt-task.json".into(),
            Edit::Locate => String::new(),
            Edit::Skill => self
                .args
                .skill_agent
                .unwrap_or_default()
                .default_path()
                .into(),
        };
        let cursor = text.len();
        self.input = Some(Input { edit, text, cursor });
    }
    fn apply_setting(&mut self, setting: Setting, value: &str) -> Result<(), String> {
        let value = value.trim();
        let mut args = self.args.clone();
        let boolean = || {
            value
                .parse::<bool>()
                .map_err(|_| "Enter true or false".to_string())
        };
        let number = || {
            if value.is_empty() || value == "0" {
                Ok(None)
            } else {
                value
                    .parse::<usize>()
                    .map(Some)
                    .map_err(|_| "Enter a positive integer or 0 for unlimited".to_string())
            }
        };
        let split = |separator| {
            value
                .split(separator)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        match setting {
            Setting::Paths => {
                args.paths = split(';').into_iter().map(PathBuf::from).collect();
                args.stdin_paths = false;
            }
            Setting::Git => {
                args.diff = false;
                args.git = (!value.is_empty() && value != "all").then(|| value.into());
                if args.git.is_none() {
                    args.impact = false;
                }
            }
            Setting::Query => args.query = (!value.is_empty()).then(|| value.into()),
            Setting::Ranker => args.ranker = crate::query::Ranker::from_str(value, true)?,
            Setting::NoQueryExpansion => args.no_query_expansion = boolean()?,
            Setting::Budget => args.max_tokens = Some(number()?.unwrap_or(0)),
            Setting::Model => args.model = Some(value.into()),
            Setting::Context => args.context = crate::symbols::ContextLevel::from_str(value, true)?,
            Setting::Symbol => args.symbol = (!value.is_empty()).then(|| value.into()),
            Setting::NoCache => args.no_cache = boolean()?,
            Setting::CacheDir => args.cache_dir = (!value.is_empty()).then(|| PathBuf::from(value)),
            Setting::Scanner => {
                args.secret_scanner = crate::security::SecretScanner::from_str(value, true)?
            }
            Setting::SecretAllow => args.secret_allow = split(';'),
            Setting::WorkspaceRoot => {
                args.workspace_root = (!value.is_empty()).then(|| PathBuf::from(value))
            }
            Setting::Format => args.format = Some(OutputFormat::from_str(value, true)?),
            Setting::SkillAgent => {
                self.args.skill_agent = Some(crate::skill::SkillAgent::from_str(value, true)?);
                self.status =
                    "Skill location preference updated; press E to export the guide".into();
                return Ok(());
            }
            Setting::PathStyle => args.path_style = Some(PathStyle::from_str(value, true)?),
            Setting::Secrets => args.secrets = Some(SecretPolicy::from_str(value, true)?),
            Setting::Minify => {
                args.minify = boolean()?;
                args.no_minify = !args.minify;
            }
            Setting::Trace => args.trace = boolean()?,
            Setting::Impact => {
                args.impact = boolean()?;
                if args.impact && !(args.diff || args.git.is_some()) {
                    return Err("Choose a Git mode before enabling impact".into());
                }
            }
            Setting::Manifest => args.manifest = boolean()?,
            Setting::Include => args.include = split(';'),
            Setting::Ignore => args.ignore = split(';'),
            Setting::Extensions => args.extensions = split(','),
            Setting::Languages => args.languages = split(','),
            Setting::Hidden => args.exclude_hidden = boolean()?,
            Setting::MaxFiles => args.max_files = Some(number()?.unwrap_or(0)),
            Setting::NoLimit => args.no_limit = boolean()?,
            Setting::AllowBroad => args.allow_broad_scan = boolean()?,
            Setting::ScanEntries => args.scan_max_entries = Some(number()?.unwrap_or(0)),
            Setting::ScanMb => args.scan_max_mb = Some(number()?.unwrap_or(0) as u64),
            Setting::ScanDepth => args.scan_max_depth = Some(number()?.unwrap_or(0)),
            Setting::ScanTimeout => args.scan_timeout = Some(number()?.unwrap_or(0) as u64),
            Setting::Destination => {
                args.output = None;
                args.stdout = false;
                args.clipboard = false;
                args.ci = false;
                match value {
                    "file" => args.output = Some("context.md".into()),
                    "stdout" => {
                        args.stdout = true;
                        args.compression = Compression::None;
                    }
                    "clipboard" => {
                        args.clipboard = true;
                        args.compression = Compression::None;
                    }
                    _ => return Err("Choose clipboard, stdout, or file".into()),
                }
            }
            Setting::OutputPath => {
                args.output = (!value.is_empty()).then(|| value.into());
                args.stdout = false;
                args.clipboard = false;
                if args.output.is_none() {
                    args.compression = Compression::None;
                }
            }
            Setting::Compression => {
                args.compression = Compression::from_str(value, true)?;
                if args.compression != Compression::None && args.output.is_none() {
                    return Err("Choose a file destination first".into());
                }
            }
            Setting::DryRun => args.dry_run = boolean()?,
            Setting::Ci => {
                args.ci = boolean()?;
                if args.ci {
                    args.clipboard = false;
                }
            }
            Setting::Wrap => {
                self.wrap = boolean()?;
                return Ok(());
            }
        }
        self.args = args;
        self.pending_session = None;
        self.status = format!("Applying {}…", setting.name());
        if !self.busy
            && self.workspace.is_some()
            && matches!(
                setting,
                Setting::Format
                    | Setting::PathStyle
                    | Setting::Manifest
                    | Setting::Compression
                    | Setting::DryRun
                    | Setting::Ci
            )
        {
            self.replan();
        } else {
            self.load();
        }
        Ok(())
    }
    fn apply_input(&mut self) {
        let Some(input) = self.input.take() else {
            return;
        };
        if matches!(input.edit, Edit::Save) && self.busy {
            self.status = "Wait for refresh before saving this task".into();
            self.input = Some(input);
            return;
        }
        let result = match input.edit {
            Edit::Setting(setting) => self.apply_setting(setting, &input.text),
            Edit::Search => {
                self.filter = input.text.clone();
                self.cursor = 0;
                Ok(())
            }
            Edit::Locate => self.locate(&input.text),
            Edit::Skill => {
                if input.text.trim().is_empty() {
                    Err("Choose a file path for the skill guide".into())
                } else {
                    crate::skill::export(&self.cwd, Some(Path::new(&input.text)), self.args.skill_agent.unwrap_or_default())
                        .map(|path| self.status = format!("Skill exported: {}. Reload/import in your agent; no configuration changed.", path.display()))
                }
            }
            Edit::Save => self
                .workspace
                .as_ref()
                .ok_or_else(|| "Workspace is still loading".into())
                .and_then(|w| {
                    TaskSession::capture(w, &self.args, &self.selected, &self.pinned)
                        .save(&self.cwd.join(&input.text))
                })
                .map(|_| {
                    self.status =
                        "Task saved (options and fingerprints; no source contents)".into();
                }),
            Edit::Load => {
                let root = self
                    .workspace
                    .as_ref()
                    .map(|w| w.prepared.root.as_path())
                    .unwrap_or(&self.cwd);
                TaskSession::load(&self.cwd.join(&input.text), root).map(|session| {
                    self.args = session.args.clone();
                    self.args.tui = false;
                    self.args.stdin_paths = false;
                    self.pending_session = Some(session);
                    self.load();
                })
            }
        };
        if let Err(error) = result {
            self.status = error;
            self.input = Some(input);
        }
    }
    fn locate(&mut self, text: &str) -> Result<(), String> {
        let mut path = text.trim();
        let mut numbers = Vec::new();
        while let Some((prefix, suffix)) = path.rsplit_once(':') {
            if let Ok(number) = suffix.parse::<u16>() {
                numbers.push(number);
                path = prefix;
                if numbers.len() == 2 {
                    break;
                }
            } else {
                break;
            }
        }
        let target = PathBuf::from(path);
        if (self.args.minify
            || self.args.context != crate::symbols::ContextLevel::Complete
            || self.args.symbol.is_some())
            && !numbers.is_empty()
        {
            return Err("Disable Minify in Settings for original-source line navigation; transformed line numbers differ".into());
        }
        let workspace = self
            .workspace
            .as_ref()
            .ok_or("Workspace is still loading")?;
        let found = workspace.locate_paths(&target);
        if found.len() != 1 {
            return Err("Use an unambiguous file path within the current candidates; adjust paths/filters in Settings if needed".into());
        }
        let found = &found[0];
        if !numbers.is_empty()
            && workspace
                .files
                .iter()
                .any(|f| &f.file.path == found && f.file.redactions > 0)
        {
            return Err("Redacted preview line numbers may differ; use original source spans rather than transformed line navigation".into());
        }
        self.panel = Panel::Source;
        self.filter.clear();
        self.collapsed.clear();
        self.cursor = self
            .rows()
            .iter()
            .position(|row| matches!(row, Row::File(i) if &workspace.files[*i].file.path == found))
            .unwrap_or(0);
        self.scroll = (numbers.last().copied().unwrap_or(1).saturating_sub(1), 0);
        self.preview_focus = true;
        Ok(())
    }
    fn preview(&self) -> Cow<'_, str> {
        let Some(workspace) = &self.workspace else {
            return if self.busy {
                "Scanning files, resolving imports and preparing safe previews…\nYou can quit with q.".into()
            } else {
                format!("{}\n\nNo context can be exported. Press 6 for Settings to narrow input paths or adjust scan limits, then refresh with r. q quits.", self.status).into()
            };
        };
        match self.panel {
            Panel::Checks => {
                let mut text = vec![
                    "FINAL HANDOFF CHECKS".into(),
                    format!("Dependency index: {} reused / {} rebuilt / {} uncached", workspace.graph_stats.reused, workspace.graph_stats.rebuilt, workspace.graph_stats.uncacheable),
                    "Packet token count includes rendered metadata; exact for cl100k/o200k, estimated for other profiles.".into(),
                    String::new(),
                ];
                if let Some(plan) = &self.plan {
                    if let Ok(result) = &plan.result {
                        text.push(format!(
                            "Exports {} files / {}{} tokens / {} redactions",
                            result.packed_files.len(),
                            if result.tokens_exact { "" } else { "~" },
                            result.estimated_tokens,
                            result.redactions
                        ));
                    }
                    text.extend(plan.checks.iter().cloned());
                } else {
                    text.push("Updating final plan…".into());
                }
                text.extend(workspace.prepared.skipped_details.iter().map(|s| {
                    format!(
                        "Skipped {}: {}",
                        s.path
                            .as_ref()
                            .map(|p| workspace.relative(p))
                            .unwrap_or_default(),
                        s.reason
                    )
                }));
                text.join("\n").into()
            }
            Panel::Output => self
                .plan
                .as_ref()
                .map(|p| match &p.result {
                    Ok(result) => Cow::Borrowed(result.rendered.as_str()),
                    Err(error) => Cow::Borrowed(error.as_str()),
                })
                .unwrap_or_else(|| "Updating final output…".into()),
            _ => {
                let Some(path) = self.current_path() else {
                    return if self.panel == Panel::Changes {
                        "No changes for this comparison. Set Git selection in Settings.\nDeletions appear here; export uses current checkout contents.".into()
                    } else {
                        "No matching files. Adjust filters or Settings.".into()
                    };
                };
                if self.panel == Panel::Dependencies {
                    return workspace.relationships(&path).join("\n").into();
                }
                if self.panel == Panel::Symbols {
                    return workspace.files.iter().find(|f| f.file.path == path).map(|f| {
                        let mut rows = vec!["AST SYMBOLS — original source locations; y selects a name, Settings changes context level.".to_string()];
                        rows.extend(f.symbols.iter().map(|s| format!("{}  lines {}–{}\n  {}\n", s.qualified_name, s.span.start_line, s.span.end_line, s.signature)));
                        if f.symbols.is_empty() { rows.push("No symbols for this grammar/parse. Complete-source fallback is available.".into()); }
                        Cow::Owned(rows.join("\n"))
                    }).unwrap_or_else(|| "Select a file to inspect its symbols.".into());
                }
                if self.panel == Panel::Changes {
                    if let Some(change) = workspace.changes.iter().find(|c| c.path == path) {
                        if change.status == "??" {
                            return workspace
                                .files
                                .iter()
                                .find(|f| f.file.path == path)
                                .map(|s| Cow::Borrowed(s.file.content.as_str()))
                                .unwrap_or_else(|| {
                                    "Untracked file excluded by active filters or safety limits"
                                        .into()
                                });
                        }
                        return Cow::Borrowed(change.diff.as_str());
                    }
                }
                if let Some(snapshot) = workspace.files.iter().find(|f| f.file.path == path) {
                    return Cow::Borrowed(snapshot.file.content.as_str());
                }
                "Directory: Space selects subtree; Enter folds it.\nDependencies shows file import relationships.".into()
            }
        }
    }
}

pub fn run(args: &Args, cwd: &Path) -> Result<Option<TuiSelection>, String> {
    let mut terminal = ratatui::try_init().map_err(|e| e.to_string())?;
    let _ = crossterm::execute!(io::stdout(), EnableMouseCapture, EnableBracketedPaste);
    let mut app = App::new(args.clone(), cwd);
    let result = (|| loop {
        if let Some(selection) = app.poll() {
            return Ok(Some(selection));
        }
        app.tick = app.tick.wrapping_add(1);
        terminal
            .draw(|frame| render(frame, &mut app))
            .map_err(|e| e.to_string())?;
        if !event::poll(Duration::from_millis(80)).map_err(|e| e.to_string())? {
            continue;
        }
        match event::read().map_err(|e| e.to_string())? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(None);
                }
                if let Some(input) = &mut app.input {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('u')
                    {
                        input.text.clear();
                        input.cursor = 0;
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc => app.input = None,
                        KeyCode::Enter => app.apply_input(),
                        KeyCode::Backspace => {
                            let end = input.cursor;
                            input.left();
                            input.text.replace_range(input.cursor..end, "");
                        }
                        KeyCode::Delete => {
                            let start = input.cursor;
                            input.right();
                            input.text.replace_range(start..input.cursor, "");
                            input.cursor = start;
                        }
                        KeyCode::Left => input.left(),
                        KeyCode::Right => input.right(),
                        KeyCode::Home => input.cursor = 0,
                        KeyCode::End => input.cursor = input.text.len(),
                        KeyCode::Up => input.choose(-1),
                        KeyCode::Down => input.choose(1),
                        KeyCode::Char(c) => input.insert(&c.to_string()),
                        _ => {}
                    }
                    continue;
                }
                if app.help {
                    if matches!(
                        key.code,
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
                    ) {
                        app.help = false;
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => return Ok(None),
                    KeyCode::Char('1'..='7') => {
                        if let KeyCode::Char(c) = key.code {
                            app.switch(PANELS[c as usize - '1' as usize]);
                        }
                    }
                    KeyCode::Tab => {
                        let i = PANELS.iter().position(|p| p == &app.panel).unwrap_or(0);
                        app.switch(PANELS[(i + 1) % PANELS.len()]);
                    }
                    KeyCode::Char('o') | KeyCode::F(2) => app.switch(Panel::Settings),
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('b') => app.edit(Edit::Setting(Setting::Budget)),
                    KeyCode::Char('e') => app.edit(Edit::Setting(Setting::Query)),
                    KeyCode::Char('y') => app.edit(Edit::Setting(Setting::Symbol)),
                    KeyCode::Char('c') => app.edit(Edit::Setting(Setting::Git)),
                    KeyCode::Char('/') => app.edit(Edit::Search),
                    KeyCode::Char('F') => app.edit(Edit::Locate),
                    KeyCode::Char('S') => app.edit(Edit::Save),
                    KeyCode::Char('L') => app.edit(Edit::Load),
                    KeyCode::Char('E') => app.edit(Edit::Skill),
                    KeyCode::Char('r') => app.load(),
                    KeyCode::Char('P') => app.pin(),
                    KeyCode::Char('U') if !app.busy => {
                        app.pinned.clear();
                        app.status = "Pins cleared".into();
                        app.replan();
                    }
                    KeyCode::Char(' ') => app.toggle(),
                    KeyCode::Char('g') => {
                        if app.busy || app.plan.is_none() {
                            app.status = "Wait for the final plan before exporting".into();
                        } else if app.selected.is_empty() {
                            app.status = "Select at least one file".into();
                        } else if let Some(workspace) = app.workspace.clone() {
                            match app.plan.as_ref().unwrap().result.clone() {
                                Ok(result) => app.submit(Work::Export(
                                    workspace,
                                    app.args.clone(),
                                    app.selected.clone(),
                                    app.pinned.clone(),
                                    result,
                                )),
                                Err(error) => {
                                    app.status = error;
                                    app.switch(Panel::Checks);
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if app.panel == Panel::Settings {
                            app.edit(Edit::Setting(SETTINGS[app.settings_cursor]));
                        } else {
                            app.fold();
                        }
                    }
                    KeyCode::Char('v') => app.wrap = !app.wrap,
                    KeyCode::Char('w') => app.preview_focus = !app.preview_focus,
                    KeyCode::Char('J') => app.scroll.0 = app.scroll.0.saturating_add(5),
                    KeyCode::Char('K') => app.scroll.0 = app.scroll.0.saturating_sub(5),
                    KeyCode::Left if app.preview_focus => {
                        app.scroll.1 = app.scroll.1.saturating_sub(4)
                    }
                    KeyCode::Right if app.preview_focus => {
                        app.scroll.1 = app.scroll.1.saturating_add(4)
                    }
                    KeyCode::Left | KeyCode::Right => app.fold(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.preview_focus {
                            app.scroll.0 = app.scroll.0.saturating_sub(1);
                        } else {
                            app.move_cursor(-1);
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.preview_focus {
                            app.scroll.0 = app.scroll.0.saturating_add(1);
                        } else {
                            app.move_cursor(1);
                        }
                    }
                    KeyCode::PageUp => {
                        if app.preview_focus {
                            app.scroll.0 = app.scroll.0.saturating_sub(10);
                        } else {
                            app.move_cursor(-10);
                        }
                    }
                    KeyCode::PageDown => {
                        if app.preview_focus {
                            app.scroll.0 = app.scroll.0.saturating_add(10);
                        } else {
                            app.move_cursor(10);
                        }
                    }
                    KeyCode::Home => {
                        app.cursor = 0;
                        app.settings_cursor = 0;
                        app.scroll = (0, 0);
                    }
                    KeyCode::End => {
                        app.cursor = app.rows().len().saturating_sub(1);
                        app.settings_cursor = SETTINGS.len() - 1;
                    }
                    KeyCode::Char('a' | 'n') if !app.busy => {
                        if let Some(workspace) = &app.workspace {
                            for file in &workspace.files {
                                if app.matches(&workspace.relative(&file.file.path)) {
                                    if key.code == KeyCode::Char('a') {
                                        app.selected.insert(file.file.path.clone());
                                    } else {
                                        app.selected.remove(&file.file.path);
                                        app.pinned.remove(&file.file.path);
                                    }
                                }
                            }
                        }
                        app.replan();
                    }
                    KeyCode::Esc => {
                        app.filter.clear();
                        app.cursor = 0;
                    }
                    _ => {}
                }
            }
            Event::Paste(text) => {
                if let Some(input) = &mut app.input {
                    input.insert(&text.replace(['\r', '\n'], " "));
                }
            }
            Event::Mouse(mouse) if app.input.is_none() && !app.help => match mouse.kind {
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                    let delta = if mouse.kind == MouseEventKind::ScrollDown {
                        3
                    } else {
                        -3
                    };
                    if contains(app.preview_area, mouse.column, mouse.row) {
                        app.scroll.0 = app.scroll.0.saturating_add_signed(delta as i16);
                    } else {
                        app.move_cursor(delta);
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((panel, _)) = app
                        .tab_hits
                        .iter()
                        .find(|(_, r)| contains(*r, mouse.column, mouse.row))
                    {
                        let panel = *panel;
                        app.switch(panel);
                    } else if app.panel == Panel::Settings
                        && contains(app.settings_area, mouse.column, mouse.row)
                    {
                        app.settings_cursor = (app.settings_list.offset()
                            + mouse.row.saturating_sub(app.settings_area.y + 1) as usize)
                            .min(SETTINGS.len() - 1);
                        app.edit(Edit::Setting(SETTINGS[app.settings_cursor]));
                    } else if contains(app.tree_area, mouse.column, mouse.row) {
                        app.preview_focus = false;
                        app.cursor = (app.list.offset()
                            + mouse.row.saturating_sub(app.tree_area.y + 1) as usize)
                            .min(app.rows().len().saturating_sub(1));
                        app.scroll = (0, 0);
                        if mouse.column <= app.tree_area.x + 5 {
                            app.toggle();
                        }
                    } else if contains(app.preview_area, mouse.column, mouse.row) {
                        app.preview_focus = true;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    })();
    let _ = crossterm::execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::try_restore().map_err(|e| e.to_string())?;
    result
}

const ACCENT: Color = Color::Rgb(91, 214, 200);
fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(
            Style::default()
                .bg(Color::Rgb(16, 20, 28))
                .fg(Color::Rgb(215, 221, 232)),
        ),
        area,
    );
    if area.width < 52 || area.height < 15 {
        frame.render_widget(
            Paragraph::new("cxt — enlarge terminal to at least 52 × 15. q quits.")
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let layout = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Length(3),
    ])
    .split(area);
    let activity = if app.busy {
        ["◐", "◓", "◑", "◒"][app.tick / 2 % 4]
    } else {
        "●"
    };
    let summary = app
        .plan
        .as_ref()
        .and_then(|p| p.result.as_ref().ok())
        .map(|r| {
            format!(
                "{} files / {}{} tokens / {} redactions",
                r.packed_files.len(),
                if r.tokens_exact { "" } else { "~" },
                r.estimated_tokens,
                r.redactions
            )
        })
        .unwrap_or_else(|| "Planning…".into());
    let root = app
        .workspace
        .as_ref()
        .map(|w| w.root.display().to_string())
        .unwrap_or_else(|| app.cwd.display().to_string());
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    " cxt ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {}  {}", activity, summary)),
            ]),
            Line::from(format!(" {}", root)),
        ]),
        Rect::new(layout[0].x, layout[0].y, layout[0].width, 2),
    );
    app.tab_hits.clear();
    let mut x = layout[0].x;
    let mut spans = Vec::new();
    for panel in PANELS {
        let label = if area.width < 100 {
            panel.title().split_whitespace().next().unwrap_or("")
        } else {
            panel.title()
        };
        let label = format!(" {} ", label);
        let width = label.chars().count() as u16;
        app.tab_hits
            .push((*panel, Rect::new(x, layout[0].y + 2, width, 1)));
        spans.push(Span::styled(
            label,
            if *panel == app.panel {
                Style::default().fg(Color::Black).bg(ACCENT)
            } else {
                Style::default().fg(Color::Gray)
            },
        ));
        x += width;
    }
    spans.push(Span::raw(format!(
        "  {}",
        app.panel
            .title()
            .split_once(' ')
            .map(|(_, s)| s)
            .unwrap_or("")
    )));
    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(layout[0].x, layout[0].y + 2, layout[0].width, 1),
    );
    if app.panel == Panel::Settings {
        render_settings(frame, app, layout[1]);
    } else {
        let body = if area.width < 100 {
            Layout::vertical([Constraint::Percentage(38), Constraint::Percentage(62)])
                .split(layout[1])
        } else {
            Layout::horizontal([Constraint::Percentage(36), Constraint::Percentage(64)])
                .split(layout[1])
        };
        app.tree_area = body[0];
        app.preview_area = body[1];
        app.settings_area = Rect::default();
        let rows = app.rows();
        app.cursor = app.cursor.min(rows.len().saturating_sub(1));
        let items: Vec<_> = rows
            .iter()
            .map(|row| {
                let workspace = app.workspace.as_ref().unwrap();
                match row {
                    Row::Directory(path) => {
                        let prefix = if app.collapsed.contains(path) {
                            "▸"
                        } else {
                            "▾"
                        };
                        ListItem::new(format!(" {} {}", prefix, workspace.relative(path)))
                            .style(Style::default().fg(ACCENT))
                    }
                    Row::File(i) => {
                        let file = &workspace.files[*i].file;
                        let chosen = if app.selected.contains(&file.path) {
                            "●"
                        } else {
                            "○"
                        };
                        let pin = if app.pinned.contains(&file.path) {
                            "★"
                        } else {
                            " "
                        };
                        let excluded = app
                            .plan
                            .as_ref()
                            .is_some_and(|p| p.excluded.contains(&file.path));
                        ListItem::new(format!(
                            " {}{} {} {}  ~{}{}",
                            chosen,
                            pin,
                            workspace
                                .statuses
                                .get(&file.path)
                                .map(String::as_str)
                                .unwrap_or(""),
                            workspace.relative(&file.path),
                            file.tokens,
                            if excluded { " !" } else { "" }
                        ))
                        .style(Style::default().fg(if excluded {
                            Color::Yellow
                        } else {
                            Color::White
                        }))
                    }
                    Row::Change(i) => {
                        let change = &workspace.changes[*i];
                        ListItem::new(format!(
                            " {} {}",
                            change.status,
                            workspace.relative(&change.path)
                        ))
                        .style(Style::default().fg(
                            if change.status == "D" {
                                Color::Red
                            } else {
                                ACCENT
                            },
                        ))
                    }
                }
            })
            .collect();
        app.list.select((!rows.is_empty()).then_some(app.cursor));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(if app.panel == Panel::Changes {
                            " Changes (current checkout export) "
                        } else {
                            " Files  ● selected  ★ pinned  ! excluded "
                        })
                        .borders(Borders::ALL),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(42, 52, 68))
                        .add_modifier(Modifier::BOLD),
                ),
            body[0],
            &mut app.list,
        );
        let preview = app.preview();
        let lines: Vec<_> = preview
            .lines()
            .enumerate()
            .skip(app.scroll.0 as usize)
            .take(body[1].height as usize)
            .map(|(i, line)| {
                let color = if app.panel == Panel::Changes && line.starts_with('+') {
                    Color::Green
                } else if app.panel == Panel::Changes && line.starts_with('-') {
                    Color::Red
                } else if line.starts_with("@@") {
                    ACCENT
                } else {
                    Color::White
                };
                let prefix = if app.panel == Panel::Source {
                    format!("{:>4} │ ", i + 1)
                } else {
                    String::new()
                };
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                    Span::styled(line.to_string(), Style::default().fg(color)),
                ])
            })
            .collect();
        let reason = app
            .current_path()
            .and_then(|p| {
                app.workspace
                    .as_ref()?
                    .files
                    .iter()
                    .find(|s| s.file.path == p)
                    .map(|s| s.file.reasons.join(", "))
            })
            .unwrap_or_default();
        let mut paragraph = Paragraph::new(lines).scroll((0, app.scroll.1)).block(
            Block::default()
                .title(format!(
                    " {} {} ",
                    app.panel.title(),
                    if app.preview_focus {
                        "[focused]"
                    } else {
                        "w focuses"
                    }
                ))
                .title_bottom(reason)
                .borders(Borders::ALL),
        );
        if app.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        frame.render_widget(paragraph, body[1]);
    }
    frame.render_widget(Paragraph::new(vec![Line::from(Span::styled(app.status.clone(), Style::default().fg(Color::Yellow))), Line::from("Tab/1–7 views  Space select  P pin  e query  y symbol  b budget  g export  E skill  ? help  q quit")]).wrap(Wrap { trim: false }), layout[2]);
    if app.help {
        popup(frame, "Keyboard & mouse", "Tab or 1–7: workflow views    o/F2: all settings\n↑↓ / j k: navigate    Enter/←→: fold directories\nSpace: select file/subtree    a/n: select all/none matching filter\nP: pin essential file (export blocked if it cannot fit)\n/: filter paths    e: task query    y: symbol    b: budget    c: Git selection\n7: original-source symbol catalog; context level is editable in Settings\nw: focus preview    J/K: scroll    ←→: horizontal scroll when focused\nv: wrap preview    mouse wheel: scroll hovered pane\nF: locate file:line[:column] in complete, unminified, unredacted source\nS: save task to a new filename    L: load task\nr: refresh sources    g: export exact final plan after freshness check\nClick tabs/settings/checkboxes. Paste is supported in input dialogs.\nEsc cancels a dialog; q exits. cl100k/o200k count exact packet tokens; test suggestions remain heuristic.");
    }
    if let Some(input) = &app.input {
        let (title, hint) = match input.edit {
            Edit::Setting(s) => (s.name(), s.hint()),
            Edit::Search => (
                "Filter paths",
                "Substring terms; Enter applies, Esc cancels",
            ),
            Edit::Save => (
                "Save task",
                "Use a new filename. Saves options and hashes, never source contents.",
            ),
            Edit::Load => (
                "Load task",
                "Restore a task from this workspace; changed files are reported.",
            ),
            Edit::Locate => (
                "Locate failure",
                "Paste file:line[:column], including Windows drive paths.",
            ),
            Edit::Skill => (
                "Export agent SKILL.md",
                "Choose a new file path. Agent preference is in Settings. Static guide only; existing files are never overwritten.",
            ),
        };
        popup(
            frame,
            title,
            &format!(
                "{}\n\n{}█{}\n\nEnter applies • Esc cancels • Ctrl+u clears • ↑↓ chooses\n{}",
                hint,
                &input.text[..input.cursor],
                &input.text[input.cursor..],
                app.status
            ),
        );
    }
}
fn render_settings(frame: &mut Frame, app: &mut App, area: Rect) {
    app.settings_area = area;
    app.tree_area = Rect::default();
    app.preview_area = Rect::default();
    let items: Vec<_> = SETTINGS
        .iter()
        .map(|setting| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {:<34} ", setting.name()),
                    Style::default().fg(ACCENT),
                ),
                Span::raw(app.setting_value(*setting)),
            ]))
        })
        .collect();
    app.settings_list.select(Some(app.settings_cursor));
    frame.render_stateful_widget(List::new(items).block(Block::default().title(" Settings — Enter or click to edit ").title_bottom("Project/global include and ignore rules still apply; edit .cxt.toml to change them.").borders(Borders::ALL)).highlight_style(Style::default().bg(Color::Rgb(42, 52, 68))), area, &mut app.settings_list);
}
fn popup(frame: &mut Frame, title: &str, text: &str) {
    let area = frame.area();
    let width = area.width.saturating_sub(4).min(94);
    let height = area.height.saturating_sub(4).min(21);
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(27, 34, 46)).fg(Color::White)),
        ),
        popup,
    );
}
fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.right() && y >= rect.y && y < rect.bottom()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use ratatui::{backend::TestBackend, Terminal};
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "cxt-tui-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::write(path.join("main.rs"), "fn main() {}\n").unwrap();
            Self(path)
        }
        fn app(&self) -> App {
            let mut app = App::new(Args::parse_from(["cxt"]).normalized(), &self.0);
            ready(&mut app);
            app
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn ready(app: &mut App) {
        let start = Instant::now();
        while app.busy {
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "worker stalled at revision {} (workspace={}, plan={}): {}",
                app.revision,
                app.workspace.is_some(),
                app.plan.is_some(),
                app.status
            );
            app.poll();
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn all_workflow_views_and_dialogs_render_at_supported_sizes() {
        let f = Fixture::new();
        let mut app = f.app();
        for (width, height) in [(52, 15), (80, 24), (140, 40)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            for panel in PANELS {
                app.switch(*panel);
                terminal.draw(|frame| render(frame, &mut app)).unwrap();
                let screen: String = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|c| c.symbol())
                    .collect();
                assert!(screen.contains("cxt"));
                assert!(screen.contains(panel.title().split_once(' ').unwrap().1));
            }
            for edit in [
                Edit::Setting(Setting::Secrets),
                Edit::Setting(Setting::SkillAgent),
                Edit::Skill,
            ] {
                app.edit(edit);
                terminal.draw(|frame| render(frame, &mut app)).unwrap();
                app.input = None;
            }
        }
    }

    #[test]
    fn full_source_and_failure_location_work_beyond_old_preview_limit() {
        let f = Fixture::new();
        let mut text = "// line\n".repeat(300);
        text.push_str("fn last_function() {}\n");
        fs::write(f.0.join("main.rs"), text).unwrap();
        let mut app = f.app();
        app.locate("main.rs:301:4").unwrap();
        assert_eq!(app.scroll.0, 300);
        assert!(app.preview().contains("last_function"));
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(screen.contains("last_function"));
    }

    #[test]
    fn setting_edits_apply_latest_options_and_override_project_budget() {
        let f = Fixture::new();
        fs::write(f.0.join(".cxt.toml"), "max_tokens = 1\n").unwrap();
        let mut app = f.app();
        assert!(app.plan.as_ref().unwrap().result.is_err());
        app.apply_setting(Setting::Budget, "0").unwrap();
        app.apply_setting(Setting::Destination, "file").unwrap();
        app.apply_setting(Setting::Compression, "gzip").unwrap();
        ready(&mut app);
        assert_eq!(app.args.max_tokens, Some(0));
        assert!(app.args.output.is_some());
        assert_eq!(app.args.compression, Compression::Gzip);
        assert!(app.plan.as_ref().unwrap().result.is_ok());
        assert!(app.apply_setting(Setting::Secrets, "invalid").is_err());
        assert!(app.apply_setting(Setting::Budget, "-1").is_err());
    }

    #[test]
    fn worker_exports_exact_reviewed_result_and_blocks_stale_sources() {
        let f = Fixture::new();
        let args = Args::parse_from(["cxt", "--manifest"]).normalized();
        let workspace = Arc::new(Workspace::load(&args, &f.0).unwrap());
        let selected: BTreeSet<_> = workspace
            .files
            .iter()
            .map(|s| s.file.path.clone())
            .collect();
        let plan = workspace.plan(&args, &f.0, &selected, &BTreeSet::new());
        let reviewed = plan.result.unwrap();
        let (tx, rx) = worker(f.0.clone());
        tx.send(Request {
            revision: 1,
            work: Work::Export(
                workspace.clone(),
                args.clone(),
                selected.clone(),
                BTreeSet::new(),
                reviewed.clone(),
            ),
        })
        .unwrap();
        let response = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(response.export.unwrap().result.rendered, reviewed.rendered);
        fs::write(f.0.join("main.rs"), "fn changed() {}\n").unwrap();
        tx.send(Request {
            revision: 2,
            work: Work::Export(workspace, args, selected, BTreeSet::new(), reviewed),
        })
        .unwrap();
        let response = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(response.export.is_none());
        assert!(response.error.unwrap().contains("refresh"));
    }

    #[test]
    fn obsolete_responses_do_not_replace_current_workspace_state() {
        let f = Fixture::new();
        let mut app = f.app();
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        tx.send(Response {
            revision: app.revision.saturating_sub(1),
            workspace: None,
            plan: None,
            export: None,
            error: Some("obsolete error".into()),
        })
        .unwrap();
        app.poll();
        assert_ne!(app.status, "obsolete error");
        assert!(app.plan.is_some());
    }

    #[test]
    fn disconnected_worker_reports_failure_instead_of_spinning_forever() {
        let f = Fixture::new();
        let mut app = f.app();
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        drop(tx);
        app.busy = true;
        app.poll();
        assert!(!app.busy);
        assert!(app.plan.is_none());
        assert!(app.workspace.is_none());
        assert!(app.status.contains("Background worker stopped"));
    }

    #[test]
    fn input_cursor_and_choice_controls_support_unicode() {
        let mut input = Input {
            edit: Edit::Search,
            text: "路径.rs".into(),
            cursor: 0,
        };
        input.right();
        input.insert("新");
        assert_eq!(input.text, "路新径.rs");
        input.left();
        assert_eq!(&input.text[input.cursor..], "新径.rs");
        input.edit = Edit::Setting(Setting::Secrets);
        input.text = "redact".into();
        input.choose(1);
        assert_eq!(input.text, "fail");
        assert_eq!(input.cursor, input.text.len());
    }

    #[test]
    fn line_navigation_does_not_claim_original_locations_after_minifying() {
        let f = Fixture::new();
        let mut app = f.app();
        app.apply_setting(Setting::Minify, "true").unwrap();
        ready(&mut app);
        assert!(app
            .locate("main.rs:1")
            .unwrap_err()
            .contains("Disable Minify"));
        assert!(app.locate("main.rs").is_ok());
    }

    #[test]
    fn saved_task_waits_for_consistent_options_and_snapshots() {
        let f = Fixture::new();
        let mut app = f.app();
        app.apply_setting(Setting::Budget, "500").unwrap();
        app.edit(Edit::Save);
        app.apply_input();
        assert!(app.status.contains("Wait for refresh"));
        assert!(!f.0.join(".cxt-task.json").exists());
        ready(&mut app);
        app.apply_input();
        assert!(f.0.join(".cxt-task.json").is_file());
    }

    #[test]
    fn output_settings_preserve_reviewed_sources_and_still_block_stale_export() {
        let f = Fixture::new();
        let mut app = f.app();
        app.apply_setting(Setting::Destination, "file").unwrap();
        ready(&mut app);
        app.pinned = app.selected.clone();
        let selected = app.selected.clone();
        fs::write(f.0.join("main.rs"), "fn changed_after_review() {}\n").unwrap();
        for (setting, value) in [
            (Setting::Format, "json"),
            (Setting::PathStyle, "absolute"),
            (Setting::Manifest, "true"),
            (Setting::Compression, "gzip"),
            (Setting::DryRun, "true"),
            (Setting::Ci, "true"),
        ] {
            app.apply_setting(setting, value).unwrap();
            ready(&mut app);
            let result = app.plan.as_ref().unwrap().result.as_ref().unwrap();
            assert_eq!(result.packed_files[0].content, "fn main() {}\n");
            assert_eq!(app.selected, selected);
            assert_eq!(app.pinned, selected);
            assert_eq!(
                app.workspace.as_ref().unwrap().stale_paths(&selected).len(),
                1
            );
        }
        let result = app.plan.as_ref().unwrap().result.as_ref().unwrap().clone();
        assert_eq!(result.format, OutputFormat::Json);
        assert!(serde_json::from_str::<serde_json::Value>(&result.rendered).is_ok());
        app.submit(Work::Export(
            app.workspace.clone().unwrap(),
            app.args.clone(),
            app.selected.clone(),
            app.pinned.clone(),
            result,
        ));
        ready(&mut app);
        assert!(
            app.status.contains("changed since preview"),
            "{}",
            app.status
        );
    }

    #[test]
    fn all_settings_edit_and_feed_the_shared_final_plan() {
        let f = Fixture::new();
        let mut app = f.app();
        for (setting, value) in [
            (Setting::Paths, "."),
            (Setting::Git, "all"),
            (Setting::Query, "main"),
            (Setting::Budget, "2000"),
            (Setting::Model, "code"),
            (Setting::Format, "json"),
            (Setting::PathStyle, "absolute"),
            (Setting::Secrets, "redact"),
            (Setting::Minify, "true"),
            (Setting::Trace, "true"),
            (Setting::Impact, "false"),
            (Setting::Manifest, "true"),
            (Setting::Include, "*.rs"),
            (Setting::Ignore, "missing.*"),
            (Setting::Extensions, "rs"),
            (Setting::Languages, "rust"),
            (Setting::Hidden, "true"),
            (Setting::MaxFiles, "0"),
            (Setting::NoLimit, "false"),
            (Setting::AllowBroad, "false"),
            (Setting::ScanEntries, "1000"),
            (Setting::ScanMb, "10"),
            (Setting::ScanDepth, "20"),
            (Setting::ScanTimeout, "30"),
            (Setting::Destination, "file"),
            (Setting::OutputPath, "out.json.gz"),
            (Setting::Compression, "gzip"),
            (Setting::DryRun, "false"),
            (Setting::Ci, "true"),
            (Setting::Wrap, "false"),
        ] {
            app.apply_setting(setting, value).unwrap();
            ready(&mut app);
        }
        assert_eq!(app.setting_value(Setting::MaxFiles), "0");
        let result = app.plan.as_ref().unwrap().result.as_ref().unwrap();
        assert_eq!(result.packed_files.len(), 1);
        assert!(result.estimated_tokens <= 2000);
        assert_eq!(result.format, OutputFormat::Json);
        let json: serde_json::Value = serde_json::from_str(&result.rendered).unwrap();
        assert!(json.is_object());
        assert_eq!(app.args.compression, Compression::Gzip);
        assert!(!app.wrap);
        assert!(app.apply_setting(Setting::Impact, "true").is_err());
    }

    #[test]
    fn failed_scan_blocks_old_workspace_and_recovers_when_limit_is_raised() {
        let f = Fixture::new();
        let mut app = f.app();
        assert!(app.plan.is_some());
        app.apply_setting(Setting::ScanEntries, "1").unwrap();
        ready(&mut app);
        assert!(app.status.contains("entry limit"));
        assert!(app.preview().contains("No context can be exported"));
        assert!(app.plan.is_none());
        assert!(app.workspace.is_none());
        app.replan();
        assert!(app.plan.is_none());
        app.apply_setting(Setting::ScanEntries, "100").unwrap();
        ready(&mut app);
        assert!(app.plan.as_ref().unwrap().result.is_ok());
    }
}
