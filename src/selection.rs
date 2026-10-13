use crate::cli::Args;
use crate::git;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub score: i64,
    pub reasons: Vec<String>,
}

impl Candidate {
    pub fn new(path: PathBuf, reason: impl Into<String>) -> Self {
        Self {
            path,
            score: 0,
            reasons: vec![reason.into()],
        }
    }
}

#[derive(Debug)]
pub struct SelectionResult {
    pub root: PathBuf,
    pub candidates: Vec<Candidate>,
    pub mode: String,
    /// Non-fatal directory-walk problems (permission denied, vanished files, etc.).
    /// Selection continues and the engine surfaces these as skipped items.
    pub warnings: Vec<String>,
}

pub fn gather(args: &Args, cwd: &Path) -> Result<SelectionResult, String> {
    let mut input_paths = args.paths.clone();
    if args.stdin_paths {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line.map_err(|e| format!("failed to read stdin: {}", e))?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                input_paths.push(PathBuf::from(trimmed));
            }
        }
    }
    if input_paths.is_empty() {
        input_paths.push(PathBuf::from("."));
    }

    let resolved_inputs: Vec<PathBuf> = input_paths
        .into_iter()
        .map(|path| resolve_input(cwd, path))
        .collect();

    if args.diff || args.git.is_some() {
        let start = resolved_inputs.first().map(PathBuf::as_path).unwrap_or(cwd);
        let root = git::repository_root(start)?;
        let mode = if args.diff {
            "working".to_string()
        } else {
            args.git.clone().unwrap_or_else(|| "working".to_string())
        };
        let candidates = git::select(&root, &mode)?
            .into_iter()
            .map(|path| Candidate::new(path, format!("git:{}", mode)))
            .collect();
        return Ok(SelectionResult {
            root,
            candidates,
            mode: format!("git:{}", mode),
            warnings: Vec::new(),
        });
    }

    let root = common_root(&resolved_inputs).unwrap_or_else(|| cwd.to_path_buf());
    let mut map: BTreeMap<PathBuf, Candidate> = BTreeMap::new();
    let mut warnings = Vec::new();
    for input in resolved_inputs {
        if input.is_file() {
            insert_candidate(&mut map, input, "explicit-path");
        } else if input.is_dir() {
            scan_directory(
                &input,
                args.exclude_hidden,
                args.no_limit,
                &mut map,
                &mut warnings,
            );
        } else {
            return Err(format!("path does not exist: {}", input.display()));
        }
    }

    Ok(SelectionResult {
        root,
        candidates: map.into_values().collect(),
        mode: if args.query.is_some() {
            "query".to_string()
        } else {
            "paths".to_string()
        },
        warnings,
    })
}

pub fn insert_candidate(
    map: &mut BTreeMap<PathBuf, Candidate>,
    path: PathBuf,
    reason: impl Into<String>,
) {
    let path = path.canonicalize().unwrap_or(path);
    let reason = reason.into();
    match map.get_mut(&path) {
        Some(candidate) => {
            if !candidate.reasons.contains(&reason) {
                candidate.reasons.push(reason);
            }
        }
        None => {
            map.insert(path.clone(), Candidate::new(path, reason));
        }
    }
}

fn scan_directory(
    dir: &Path,
    exclude_hidden: bool,
    no_limit: bool,
    map: &mut BTreeMap<PathBuf, Candidate>,
    warnings: &mut Vec<String>,
) {
    let mut walker = WalkBuilder::new(dir);
    // ignore::WalkBuilder hidden(true) filters dotfiles. cxt historically included them,
    // so users opt into filtering with --exclude-hidden.
    walker.hidden(exclude_hidden);
    walker
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .skip_stdout(true);
    if !no_limit {
        walker.filter_entry(|entry| {
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git"
                    | ".hg"
                    | ".svn"
                    | "target"
                    | "node_modules"
                    | "vendor"
                    | ".venv"
                    | "venv"
                    | "__pycache__"
                    | ".cache"
                    | ".next"
                    | ".nuxt"
                    | ".turbo"
                    | "coverage"
                    | "dist"
                    | "build"
                    | "out"
            )
        });
    }

    for entry in walker.build() {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    insert_candidate(map, entry.into_path(), "directory-scan");
                }
            }
            Err(error) => {
                // A single protected/unreadable subtree must not make the entire
                // context scan fail. Keep going and surface it as a skipped item.
                warnings.push(error.to_string());
            }
        }
    }
}

fn resolve_input(cwd: &Path, path: PathBuf) -> PathBuf {
    let path = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    path.canonicalize().unwrap_or(path)
}

fn common_root(paths: &[PathBuf]) -> Option<PathBuf> {
    let first = paths.first()?;
    let mut root = if first.is_file() {
        first.parent()?.to_path_buf()
    } else {
        first.clone()
    };

    for path in paths.iter().skip(1) {
        let candidate = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path.as_path()
        };
        while !candidate.starts_with(&root) {
            if !root.pop() {
                return None;
            }
        }
    }
    Some(root)
}
