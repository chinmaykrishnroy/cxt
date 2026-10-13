use crate::cli::Args;
use crate::config::Config;
use crate::filter::FileFilter;
use crate::git;
use crate::impact;
use crate::minifier::{minify_code, strip_comments};
use crate::output::{self, OutputFormat, PackedFile, PathStyle, RenderOptions};
use crate::query;
use crate::security::{self, SecretPolicy};
use crate::selection::{self, Candidate};
use crate::token::TokenCounter;
use crate::tracer::{get_language, trace_dependencies_checked};
use crate::utils::{probe_file, FileProbe};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum EngineError {
    General(String),
    Secret(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::General(s) | Self::Secret(s) => f.write_str(s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkippedItem {
    pub path: Option<PathBuf>,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SelectionPreview {
    pub root: PathBuf,
    pub candidates: Vec<Candidate>,
    pub skipped_files: usize,
    pub skipped_details: Vec<SkippedItem>,
    pub selection_mode: String,
}

#[derive(Debug, Clone)]
pub struct EngineResult {
    pub rendered: String,
    pub packed_files: Vec<PackedFile>,
    pub selected_files: Vec<Candidate>,
    pub skipped_files: usize,
    pub skipped_details: Vec<SkippedItem>,
    pub budget_skipped: usize,
    pub estimated_tokens: usize,
    pub redactions: usize,
    pub root: PathBuf,
    pub selection_mode: String,
    pub format: OutputFormat,
}

/// A safe, transformed file and its source fingerprint. CLI and TUI use the same
/// transformation; fail-policy previews are sanitized while export is blocked.
#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub file: PackedFile,
    pub source_hash: String,
    pub secret_count: usize,
}

pub struct SnapshotOptions {
    minify: bool,
    policy: SecretPolicy,
    counter: TokenCounter,
}

impl SnapshotOptions {
    pub fn for_args(args: &Args, cwd: &Path) -> Self {
        let config = Config::load(cwd);
        Self {
            minify: !args.no_minify && (args.minify || config.default_minify.unwrap_or(false)),
            policy: args
                .secrets
                .or_else(|| {
                    config
                        .secret_policy
                        .as_deref()
                        .and_then(parse_secret_policy)
                })
                .unwrap_or_default(),
            counter: TokenCounter::for_model(
                args.model
                    .as_deref()
                    .or(config.model.as_deref())
                    .unwrap_or("heuristic"),
            ),
        }
    }
}

pub fn snapshot_file(
    args: &Args,
    cwd: &Path,
    candidate: &Candidate,
) -> Result<FileSnapshot, EngineError> {
    snapshot_file_with_options(candidate, &SnapshotOptions::for_args(args, cwd))
}

pub fn snapshot_file_with_options(
    candidate: &Candidate,
    options: &SnapshotOptions,
) -> Result<FileSnapshot, EngineError> {
    let raw =
        fs::read_to_string(&candidate.path).map_err(|e| EngineError::General(e.to_string()))?;
    let source_hash = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let secret_count = security::find_secrets(&raw).len();
    let (mut content, redactions) = if options.policy == SecretPolicy::Allow {
        (raw, 0)
    } else {
        security::redact_secrets(&raw)
    };
    let language = candidate
        .path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("text")
        .to_string();
    if options.minify {
        if let Some(lang) = get_language(&language) {
            content = strip_comments(&content, lang);
        }
        content = minify_code(&content);
    }
    let tokens = options.counter.count(&content);
    Ok(FileSnapshot {
        file: PackedFile {
            path: candidate.path.clone(),
            content,
            language,
            tokens,
            score: candidate.score,
            reasons: candidate.reasons.clone(),
            redactions,
        },
        source_hash,
        secret_count,
    })
}

pub fn snapshot_is_current(snapshot: &FileSnapshot) -> bool {
    fs::read(&snapshot.file.path)
        .is_ok_and(|raw| format!("{:x}", Sha256::digest(&raw)) == snapshot.source_hash)
}

/// Build the complete candidate set without reading/rendering every file.
///
/// The TUI uses this exact preparation path, so interactive selection has the same
/// semantics as the CLI for Git modes, query ranking, tracing, impact analysis,
/// filters, ignore rules, and file caps.
pub fn prepare(args: &Args, cwd: &Path) -> Result<SelectionPreview, EngineError> {
    if args.impact && !(args.diff || args.git.is_some()) {
        return Err(EngineError::General(
            "--impact requires --diff or --git <mode/range>".to_string(),
        ));
    }

    let config = Config::load(cwd);
    let mut selection = selection::gather(args, cwd).map_err(EngineError::General)?;
    let mut skipped_details: Vec<SkippedItem> = selection
        .warnings
        .drain(..)
        .map(|reason| SkippedItem { path: None, reason })
        .collect();

    let mut includes = config.include.clone();
    includes.extend(args.include.clone());
    let mut ignores = config.global_ignore.clone();
    ignores.extend(args.ignore.clone());

    let filter = FileFilter::new(
        selection.root.clone(),
        &includes,
        &ignores,
        &args.extensions,
        &args.languages,
        args.no_limit,
    )
    .map_err(EngineError::General)?;

    let output_abs = args.output.as_ref().map(|p| absolutize(cwd, p));
    let original_count = selection.candidates.len();
    selection.candidates.retain(|candidate| {
        if filter.should_skip(&candidate.path) {
            return false;
        }
        if let Some(output) = &output_abs {
            if same_path(&candidate.path, output) {
                return false;
            }
        }
        true
    });
    let mut skipped_files = original_count
        .saturating_sub(selection.candidates.len())
        .saturating_add(skipped_details.len());
    let mut scan_budget =
        crate::scan::ScanBudget::new(crate::scan::ScanLimits::for_args(args, cwd));
    for candidate in &selection.candidates {
        scan_budget
            .read_file(&candidate.path)
            .map_err(EngineError::General)?;
    }

    if let Some(q) = &args.query {
        let before_rank = selection.candidates.len();
        selection.candidates =
            query::rank_checked(selection.candidates, q, &selection.root, || {
                scan_budget.check_time()
            })
            .map_err(EngineError::General)?;
        skipped_files += before_rank.saturating_sub(selection.candidates.len());
        selection.mode = "query".to_string();
        if selection.candidates.is_empty() {
            return Err(EngineError::General(format!(
                "query matched no files: {}",
                q
            )));
        }
        scan_budget.check_time().map_err(EngineError::General)?;
        expand_query_dependencies(
            &mut selection.candidates,
            &filter,
            &selection.root,
            &mut scan_budget,
        )
        .map_err(EngineError::General)?;
    } else if args.trace {
        expand_trace(
            &mut selection.candidates,
            &filter,
            &selection.root,
            &mut scan_budget,
        )
        .map_err(EngineError::General)?;
        selection.mode = format!("{}+trace", selection.mode);
    }

    if args.impact {
        let seeds: Vec<PathBuf> = selection
            .candidates
            .iter()
            .map(|c| c.path.clone())
            .collect();
        let expanded =
            impact::expand(&selection.root, &seeds, args, cwd).map_err(EngineError::General)?;
        skipped_files += expanded.warnings.len();
        skipped_details.extend(
            expanded
                .warnings
                .into_iter()
                .map(|reason| SkippedItem { path: None, reason }),
        );
        merge_candidates(
            &mut selection.candidates,
            expanded
                .candidates
                .into_iter()
                .filter_map(|(path, reason)| {
                    (!filter.should_skip(&path)).then(|| {
                        let mut c = Candidate::new(path, reason);
                        c.score = 10;
                        c
                    })
                }),
        );
        selection.mode = format!("{}+impact", selection.mode);
    }

    for candidate in &selection.candidates {
        scan_budget
            .read_file(&candidate.path)
            .map_err(EngineError::General)?;
    }

    // Do this after trace/query/impact expansion so the TUI never fills with
    // permission-denied, vanished, or binary candidates showing misleading 0-token rows.
    // A bad file is non-fatal: record it and keep selecting the rest of the repository.
    let before_probe = selection.candidates.len();
    selection
        .candidates
        .retain(|candidate| match probe_file(&candidate.path) {
            FileProbe::Text => true,
            FileProbe::Binary => {
                skipped_details.push(SkippedItem {
                    path: Some(candidate.path.clone()),
                    reason: "binary file".to_string(),
                });
                false
            }
            FileProbe::Unreadable(error) => {
                skipped_details.push(SkippedItem {
                    path: Some(candidate.path.clone()),
                    reason: format!("unreadable: {}", error),
                });
                false
            }
        });
    skipped_files += before_probe.saturating_sub(selection.candidates.len());
    scan_budget.check_time().map_err(EngineError::General)?;

    if args.query.is_some() {
        selection
            .candidates
            .sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    } else {
        selection.candidates.sort_by(|a, b| {
            let a_seed = a
                .reasons
                .iter()
                .any(|r| r.starts_with("git:") || r == "explicit-path");
            let b_seed = b
                .reasons
                .iter()
                .any(|r| r.starts_with("git:") || r == "explicit-path");
            b_seed
                .cmp(&a_seed)
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| a.path.cmp(&b.path))
        });
    }

    let default_query_cap = args.query.as_ref().map(|_| 40usize);
    let file_cap = args.max_files.or(default_query_cap).filter(|cap| *cap > 0);
    if let Some(cap) = file_cap {
        if selection.candidates.len() > cap {
            skipped_files += selection.candidates.len() - cap;
            selection.candidates.truncate(cap);
        }
    }

    Ok(SelectionPreview {
        root: selection.root,
        candidates: selection.candidates,
        skipped_files,
        skipped_details,
        selection_mode: selection.mode,
    })
}

pub fn run(args: &Args, cwd: &Path) -> Result<EngineResult, EngineError> {
    let prepared = prepare(args, cwd)?;
    run_prepared(args, cwd, prepared)
}

/// Pack a selection that has already been prepared (and possibly interactively edited).
/// This is the common finalization path for both CLI and TUI workflows.
pub fn run_prepared(
    args: &Args,
    cwd: &Path,
    prepared: SelectionPreview,
) -> Result<EngineResult, EngineError> {
    run_prepared_with_progress(args, cwd, prepared, |_, _, _, _| {})
}

/// Pack a prepared selection while reporting the file currently being processed.
/// The callback is intentionally tiny and synchronous so library callers can ignore it,
/// while the CLI can attach an animated progress indicator without putting terminal I/O
/// inside `cxt_core`.
pub fn run_prepared_with_progress<F>(
    args: &Args,
    cwd: &Path,
    prepared: SelectionPreview,
    mut on_file: F,
) -> Result<EngineResult, EngineError>
where
    F: FnMut(usize, usize, &Path, u64),
{
    let mut prepared = prepared;
    let mut snapshots = Vec::new();
    let snapshot_options = SnapshotOptions::for_args(args, cwd);
    let mut budget = crate::scan::ScanBudget::new(crate::scan::ScanLimits::for_args(args, cwd));
    for (index, candidate) in prepared.candidates.iter().enumerate() {
        budget
            .read_file(&candidate.path)
            .map_err(EngineError::General)?;
        let bytes = fs::metadata(&candidate.path).map(|m| m.len()).unwrap_or(0);
        on_file(index + 1, prepared.candidates.len(), &candidate.path, bytes);
        match snapshot_file_with_options(candidate, &snapshot_options) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(error) => {
                prepared.skipped_files += 1;
                prepared.skipped_details.push(SkippedItem {
                    path: Some(candidate.path.clone()),
                    reason: format!("read failed: {}", error),
                });
            }
        }
    }
    budget.check_time().map_err(EngineError::General)?;
    run_snapshots(args, cwd, prepared, snapshots)
}

/// Finalize already transformed snapshots, including rendered overhead in the
/// budget. The TUI exports this exact result after checking source fingerprints.
pub fn run_snapshots(
    args: &Args,
    cwd: &Path,
    prepared: SelectionPreview,
    snapshots: Vec<FileSnapshot>,
) -> Result<EngineResult, EngineError> {
    if args.compression != output::Compression::None && args.output.is_none() {
        return Err(EngineError::General(
            "--compression requires --output <FILE>".to_string(),
        ));
    }

    let config = Config::load(cwd);
    let format = args
        .format
        .or_else(|| config.format.as_deref().and_then(parse_format))
        .unwrap_or_default();
    let path_style = args
        .path_style
        .or_else(|| config.path_style.as_deref().and_then(parse_path_style))
        .unwrap_or_default();
    let model = args
        .model
        .clone()
        .or(config.model.clone())
        .unwrap_or_else(|| "heuristic".to_string());
    let max_tokens = args
        .max_tokens
        .or(config.max_tokens)
        .filter(|budget| *budget > 0);
    let secret_policy = args
        .secrets
        .or_else(|| {
            config
                .secret_policy
                .as_deref()
                .and_then(parse_secret_policy)
        })
        .unwrap_or_default();
    let max_output_bytes = if args.no_limit {
        usize::MAX
    } else {
        config
            .max_output_mb
            .unwrap_or(100)
            .saturating_mul(1024 * 1024)
    };

    let SelectionPreview {
        root,
        candidates,
        skipped_files: prepared_skipped,
        skipped_details,
        selection_mode,
    } = prepared;

    let selected_files = candidates.clone();
    let counter = TokenCounter::for_model(&model);
    let mut packed = Vec::new();
    let mut content_bytes = 0usize;
    let mut content_tokens = 0usize;
    let mut budget_skipped = 0usize;
    let mut skipped_files = prepared_skipped;

    for snapshot in snapshots {
        let file = snapshot.file;
        if secret_policy == SecretPolicy::Fail && snapshot.secret_count > 0 {
            return Err(EngineError::Secret(format!(
                "secret policy 'fail' blocked {} ({} findings). Use --secrets redact to sanitize.",
                file.path.display(),
                snapshot.secret_count
            )));
        }
        let tokens = file.tokens;
        if let Some(budget) = max_tokens {
            if content_tokens + tokens > budget {
                budget_skipped += 1;
                continue;
            }
        }
        if content_bytes.saturating_add(file.content.len()) > max_output_bytes {
            skipped_files += 1;
            continue;
        }

        content_tokens += tokens;
        content_bytes += file.content.len();
        packed.push(file);
    }

    if packed.is_empty() && !selected_files.is_empty() {
        return Err(EngineError::General(
            "no selected file fit the active safety/token limits".to_string(),
        ));
    }

    let git_commit = if args.manifest {
        git::repository_root(&root)
            .ok()
            .and_then(|repo| git::head_commit(&repo))
    } else {
        None
    };

    let mut estimate_seed = content_tokens;
    let (rendered, estimated_tokens) = loop {
        let redactions: usize = packed.iter().map(|f| f.redactions).sum();
        let mut estimate_hint = estimate_seed;
        let mut iterations = 0usize;

        let (candidate_rendered, candidate_tokens) = loop {
            let options = RenderOptions {
                format,
                path_style,
                root: root.clone(),
                manifest: args.manifest,
                model: counter.model().to_string(),
                estimated_tokens: estimate_hint,
                selection_mode: selection_mode.clone(),
                query: args.query.clone(),
                git_commit: git_commit.clone(),
                redactions,
            };
            let document = output::render(&packed, &options).map_err(EngineError::General)?;
            let actual = counter.count(&document);
            iterations += 1;
            if actual == estimate_hint || !args.manifest || iterations >= 4 {
                break (document, actual);
            }
            estimate_hint = actual;
        };

        let within_tokens = max_tokens
            .map(|budget| candidate_tokens <= budget)
            .unwrap_or(true);
        let within_bytes = candidate_rendered.len() <= max_output_bytes;
        if within_tokens && within_bytes {
            break (candidate_rendered, candidate_tokens);
        }

        let Some(removed) = packed.pop() else {
            return Err(EngineError::General(
                "metadata/output overhead exceeds the configured limit".to_string(),
            ));
        };
        content_tokens = content_tokens.saturating_sub(removed.tokens);
        estimate_seed = content_tokens;
        if !within_tokens {
            budget_skipped += 1;
        } else {
            skipped_files += 1;
        }
    };

    if packed.is_empty() && !selected_files.is_empty() {
        return Err(EngineError::General(
            "no selected file fit after including rendered metadata overhead".to_string(),
        ));
    }

    let redactions = packed.iter().map(|f| f.redactions).sum();

    Ok(EngineResult {
        rendered,
        packed_files: packed,
        selected_files,
        skipped_files,
        skipped_details,
        budget_skipped,
        estimated_tokens,
        redactions,
        root,
        selection_mode,
        format,
    })
}

fn expand_query_dependencies(
    candidates: &mut Vec<Candidate>,
    filter: &FileFilter,
    root: &Path,
    budget: &mut crate::scan::ScanBudget,
) -> Result<(), String> {
    let seeds: Vec<Candidate> = candidates.iter().take(8).cloned().collect();
    let mut additions = Vec::new();
    for seed in seeds {
        let seed_label = seed
            .path
            .strip_prefix(root)
            .unwrap_or(&seed.path)
            .display()
            .to_string();
        for path in trace_dependencies_checked(
            &seed.path,
            |_| {},
            |path| {
                if filter.should_skip(path) {
                    return Ok(false);
                }
                budget.read_file(path)?;
                Ok(true)
            },
        )?
        .into_iter()
        .take(41)
        {
            if same_path(&path, &seed.path) || filter.should_skip(&path) {
                continue;
            }
            let mut candidate = Candidate::new(path, format!("dependency-of:{}", seed_label));
            candidate.score = (seed.score - 5).max(1);
            additions.push(candidate);
        }
    }
    merge_candidates(candidates, additions);
    Ok(())
}

fn expand_trace(
    candidates: &mut Vec<Candidate>,
    filter: &FileFilter,
    root: &Path,
    budget: &mut crate::scan::ScanBudget,
) -> Result<(), String> {
    let seeds: Vec<Candidate> = candidates
        .iter()
        .filter(|c| {
            c.reasons.iter().any(|r| r == "explicit-path")
                || c.reasons.iter().any(|r| r.starts_with("git:"))
        })
        .cloned()
        .collect();
    let mut additions = Vec::new();
    for seed in seeds {
        let seed_label = seed
            .path
            .strip_prefix(root)
            .unwrap_or(&seed.path)
            .display()
            .to_string();
        for path in trace_dependencies_checked(
            &seed.path,
            |_| {},
            |path| {
                if filter.should_skip(path) {
                    return Ok(false);
                }
                budget.read_file(path)?;
                Ok(true)
            },
        )? {
            if same_path(&path, &seed.path) || filter.should_skip(&path) {
                continue;
            }
            additions.push(Candidate::new(
                path,
                format!("dependency-of:{}", seed_label),
            ));
        }
    }
    merge_candidates(candidates, additions);
    Ok(())
}

fn merge_candidates<I>(candidates: &mut Vec<Candidate>, additions: I)
where
    I: IntoIterator<Item = Candidate>,
{
    let mut map: BTreeMap<PathBuf, Candidate> = BTreeMap::new();
    for mut candidate in candidates.drain(..).chain(additions) {
        let key = candidate
            .path
            .canonicalize()
            .unwrap_or_else(|_| candidate.path.clone());
        candidate.path = key.clone();
        match map.get_mut(&key) {
            Some(existing) => {
                existing.score = existing.score.max(candidate.score);
                for reason in candidate.reasons {
                    if !existing.reasons.contains(&reason) {
                        existing.reasons.push(reason);
                    }
                }
            }
            None => {
                map.insert(key, candidate);
            }
        }
    }
    *candidates = map.into_values().collect();
}

fn parse_format(value: &str) -> Option<OutputFormat> {
    match value.trim().to_ascii_lowercase().as_str() {
        "md" | "markdown" => Some(OutputFormat::Markdown),
        "xml" => Some(OutputFormat::Xml),
        "json" => Some(OutputFormat::Json),
        _ => None,
    }
}

fn parse_path_style(value: &str) -> Option<PathStyle> {
    match value.trim().to_ascii_lowercase().as_str() {
        "relative" | "rel" => Some(PathStyle::Relative),
        "absolute" | "abs" => Some(PathStyle::Absolute),
        "none" | "hidden" => Some(PathStyle::None),
        _ => None,
    }
}

fn parse_secret_policy(value: &str) -> Option<SecretPolicy> {
    match value.trim().to_ascii_lowercase().as_str() {
        "redact" => Some(SecretPolicy::Redact),
        "fail" => Some(SecretPolicy::Fail),
        "allow" => Some(SecretPolicy::Allow),
        _ => None,
    }
}

fn absolutize(cwd: &Path, path: &Path) -> PathBuf {
    let p = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    p.canonicalize().unwrap_or(p)
}

fn same_path(a: &Path, b: &Path) -> bool {
    let aa = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let bb = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    aa == bb
}
