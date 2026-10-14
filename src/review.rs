use crate::cli::Args;
use crate::engine::{self, EngineResult, FileSnapshot, SelectionPreview, SkippedItem};
use crate::{git, security};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub graph_stats: crate::dependency_index::GraphStats,
    pub cwd: PathBuf,
    pub root: PathBuf,
    pub prepared: SelectionPreview,
    pub files: Vec<FileSnapshot>,
    pub dependencies: BTreeMap<PathBuf, Vec<PathBuf>>,
    pub changes: Vec<git::Change>,
    pub statuses: BTreeMap<PathBuf, String>,
    pub head_commit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub result: Result<EngineResult, String>,
    pub checks: Vec<String>,
    pub excluded: Vec<PathBuf>,
}

impl Workspace {
    pub fn load(args: &Args, cwd: &Path) -> Result<Self, String> {
        let mut prepared = engine::prepare(args, cwd).map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        let snapshot_options = engine::SnapshotOptions::for_args(args, cwd);
        let mut budget = crate::scan::ScanBudget::new(crate::scan::ScanLimits::for_args(args, cwd));
        for candidate in &prepared.candidates {
            budget.read_file(&candidate.path)?;
            match engine::snapshot_file_with_options(candidate, &snapshot_options) {
                Ok(file) => files.push(file),
                Err(engine::EngineError::Secret(error)) => return Err(error),
                Err(error) => {
                    prepared.skipped_files += 1;
                    prepared.skipped_details.push(SkippedItem {
                        path: Some(candidate.path.clone()),
                        reason: error.to_string(),
                    });
                }
            }
        }
        budget.check_time()?;
        let root = git::repository_root(&prepared.root)
            .ok()
            .map(|p| p.canonicalize().unwrap_or(p))
            .unwrap_or_else(|| prepared.root.clone());
        let limits = crate::scan::ScanLimits::for_args(args, cwd);
        limits.guard(&root)?;
        let statuses = git::status_map(&root);
        let mode = args.git.as_deref().unwrap_or("working");
        let mut changes = if git::repository_root(&root).is_ok() {
            git::changes_with_limits(&root, mode, limits)?
        } else {
            Vec::new()
        };
        let config = crate::config::Config::for_args(args, cwd);
        let includes = config
            .include
            .into_iter()
            .chain(args.include.iter().cloned())
            .collect::<Vec<_>>();
        let ignores = config
            .global_ignore
            .into_iter()
            .chain(args.ignore.iter().cloned())
            .collect::<Vec<_>>();
        let filter = crate::filter::FileFilter::new(
            root.clone(),
            &includes,
            &ignores,
            &args.extensions,
            &args.languages,
            args.no_limit,
        )?;
        // Never show raw credentials in patch previews, even under fail policy.
        for change in &mut changes {
            change.path = change
                .path
                .canonicalize()
                .unwrap_or_else(|_| change.path.clone());
            if filter.excludes_path(&change.path) {
                change.diff = "Patch hidden by active include/ignore/safety rules.".into();
                continue;
            }
            if args.secrets != Some(security::SecretPolicy::Allow) {
                let findings = security::configured_findings(
                    &change.diff,
                    args.secret_scanner,
                    &args.secret_allow,
                )?;
                change.diff = security::redact_findings(&change.diff, &findings).0;
            }
        }
        let graph = crate::scan::graph(&root, args, cwd)?;
        prepared.skipped_files += graph.warnings.len();
        prepared.skipped_details.extend(
            graph
                .warnings
                .into_iter()
                .map(|reason| SkippedItem { path: None, reason }),
        );
        Ok(Self {
            graph_stats: graph.stats,
            cwd: cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf()),
            head_commit: git::head_commit(&root),
            root,
            prepared,
            files,
            dependencies: graph.dependencies,
            changes,
            statuses,
        })
    }

    pub fn plan(
        &self,
        args: &Args,
        cwd: &Path,
        selected: &BTreeSet<PathBuf>,
        pinned: &BTreeSet<PathBuf>,
    ) -> Plan {
        let missing_implementations: Vec<_> = self.files.iter().filter(|snapshot| {
            pinned.contains(&snapshot.file.path)
                && snapshot.file.representation == "map"
                && args.context != crate::symbols::ContextLevel::Map
        }).map(|snapshot| format!("Pinned implementation unavailable: {} — increase the budget and refresh, or choose complete context", self.relative(&snapshot.file.path))).collect();
        if !missing_implementations.is_empty() {
            return Plan {
                result: Err(
                    "pinned implementations cannot be replaced by adaptive signature maps".into(),
                ),
                checks: missing_implementations,
                excluded: selected.iter().cloned().collect(),
            };
        }
        let mut snapshots: Vec<_> = self
            .files
            .iter()
            .filter(|s| selected.contains(&s.file.path))
            .cloned()
            .collect();
        for snapshot in &mut snapshots {
            if pinned.contains(&snapshot.file.path) {
                snapshot.file.score = i64::MAX;
                snapshot.map_fallback = None;
            }
        }
        snapshots.sort_by(|a, b| {
            pinned
                .contains(&b.file.path)
                .cmp(&pinned.contains(&a.file.path))
                .then_with(|| {
                    let seed = |f: &engine::FileSnapshot| {
                        f.file
                            .reasons
                            .iter()
                            .any(|r| r.starts_with("git:") || r == "explicit-path")
                    };
                    seed(b).cmp(&seed(a))
                })
                .then_with(|| b.file.score.cmp(&a.file.score))
        });
        let mut prepared = self.prepared.clone();
        prepared.candidates = snapshots
            .iter()
            .map(|s| crate::selection::Candidate {
                path: s.file.path.clone(),
                score: s.file.score,
                reasons: s.file.reasons.clone(),
                query_symbols: Default::default(),
            })
            .collect();
        let result =
            engine::run_snapshots(args, cwd, prepared, snapshots).map_err(|e| e.to_string());
        let exported: BTreeSet<_> = result
            .as_ref()
            .map(|r| r.packed_files.iter().map(|f| f.path.clone()).collect())
            .unwrap_or_default();
        let excluded = selected.difference(&exported).cloned().collect::<Vec<_>>();
        let mut checks = Vec::new();
        if let Err(error) = &result {
            checks.push(error.clone());
        }
        for path in &excluded {
            checks.push(format!("Excluded by final plan: {}", self.relative(path)));
        }
        for path in pinned.difference(&exported) {
            checks.push(format!(
                "Pinned file cannot fit: {} — increase budget or reduce selection",
                self.relative(path)
            ));
        }
        for path in &exported {
            if let Some(deps) = self.dependencies.get(path) {
                for dep in deps.iter().filter(|p| !exported.contains(*p)) {
                    checks.push(format!(
                        "Missing dependency: {} → {}",
                        self.relative(path),
                        self.relative(dep)
                    ));
                }
            }
            if self.files.iter().any(|s| {
                &s.file.path == path && s.file.reasons.iter().any(|r| r.starts_with("git:"))
            }) && !crate::impact::looks_like_test(path)
            {
                let related = self.related_tests(path);
                if related.is_empty() {
                    checks.push(format!(
                        "No related tests resolved for {} (coverage unknown)",
                        self.relative(path)
                    ));
                } else {
                    for test in related.into_iter().filter(|test| !exported.contains(test)) {
                        checks.push(format!(
                            "Suggested test omitted: {} for {}",
                            self.relative(&test),
                            self.relative(path)
                        ));
                    }
                }
            }
        }
        let findings: usize = self
            .files
            .iter()
            .filter(|s| exported.contains(&s.file.path))
            .map(|s| s.secret_count)
            .sum();
        if findings > 0 {
            checks.push(format!(
                "{} potential secret finding(s); policy {:?}",
                findings,
                args.secrets.unwrap_or_default()
            ));
        }
        checks.push("Dependency checks cover resolved static imports; dynamic loading, call sites and test coverage may be incomplete.".into());
        if args.git.as_deref().is_some_and(|s| {
            !matches!(
                s,
                "working" | "staged" | "unstaged" | "untracked" | "tracked"
            )
        }) {
            checks.push("The comparison shows revision patches; exported file contents come from the current checkout.".into());
        }
        Plan {
            result,
            checks,
            excluded,
        }
    }

    pub fn stale_paths(&self, selected: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|s| selected.contains(&s.file.path) && !engine::snapshot_is_current(s))
            .map(|s| s.file.path.clone())
            .collect()
    }

    pub fn git_is_current(&self) -> bool {
        git::repository_root(&self.prepared.root)
            .ok()
            .and_then(|root| git::head_commit(&root))
            == self.head_commit
    }

    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string()
    }

    pub fn relationships(&self, path: &Path) -> Vec<String> {
        let mut lines = vec![format!("Imports resolved from {}", self.relative(path))];
        for dep in self.dependencies.get(path).into_iter().flatten() {
            lines.push(format!("  → {}", self.relative(dep)));
        }
        lines.push("Imported by".into());
        for (caller, deps) in &self.dependencies {
            if deps.contains(&path.to_path_buf()) {
                lines.push(format!("  ← {}", self.relative(caller)));
            }
        }
        lines.push("Related test suggestions (imports or filename heuristic)".into());
        for test in self.related_tests(path) {
            lines.push(format!("  ◇ {}", self.relative(&test)));
        }
        lines.push("These are file import relationships, not a complete symbol call graph.".into());
        lines
    }

    pub fn related_tests(&self, path: &Path) -> Vec<PathBuf> {
        let stem =
            crate::impact::normalize_stem(path.file_stem().and_then(|s| s.to_str()).unwrap_or(""));
        self.dependencies
            .iter()
            .filter(|(test, deps)| {
                if !crate::impact::looks_like_test(test) {
                    return false;
                }
                let test_stem = crate::impact::normalize_stem(
                    test.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                );
                deps.contains(&path.to_path_buf())
                    || (!stem.is_empty()
                        && !test_stem.is_empty()
                        && (test_stem.contains(&stem) || stem.contains(&test_stem)))
            })
            .map(|(test, _)| test.clone())
            .collect()
    }

    pub fn locate_paths(&self, location: &Path) -> Vec<PathBuf> {
        let normalize = |path: &Path| {
            let text = path.to_string_lossy().replace('\\', "/");
            let text = text.trim_start_matches("//?/").to_string();
            if cfg!(windows) {
                text.to_lowercase()
            } else {
                text
            }
        };
        let target = normalize(location);
        self.files
            .iter()
            .filter(|file| {
                let path = normalize(&file.file.path);
                path == target || path.ends_with(&format!("/{}", target))
            })
            .map(|file| file.file.path.clone())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskSession {
    pub schema: u32,
    pub root: PathBuf,
    pub args: Args,
    pub selected: Vec<PathBuf>,
    pub pinned: Vec<PathBuf>,
    pub fingerprints: BTreeMap<PathBuf, String>,
}

impl TaskSession {
    pub fn capture(
        workspace: &Workspace,
        args: &Args,
        selected: &BTreeSet<PathBuf>,
        pinned: &BTreeSet<PathBuf>,
    ) -> Self {
        let mut args = args.clone();
        args.paths = args
            .paths
            .iter()
            .map(|p| {
                if p.is_absolute() {
                    p.clone()
                } else {
                    workspace.cwd.join(p)
                }
            })
            .collect();
        if let Some(path) = &args.output {
            args.output = Some(if path.is_absolute() {
                path.clone()
            } else {
                workspace.cwd.join(path)
            });
        }
        Self {
            schema: 1,
            root: workspace.root.clone(),
            args,
            selected: selected.iter().cloned().collect(),
            pinned: pinned.iter().cloned().collect(),
            fingerprints: workspace
                .files
                .iter()
                .filter(|s| selected.contains(&s.file.path))
                .map(|s| (s.file.path.clone(), s.source_hash.clone()))
                .collect(),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        // Avoid silently overwriting another task or a source file.
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())
    }
    pub fn load(path: &Path, root: &Path) -> Result<Self, String> {
        let session: Self = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let root = git::repository_root(root).unwrap_or_else(|_| root.to_path_buf());
        if session.schema != 1
            || session.root.canonicalize().map_err(|e| e.to_string())?
                != root.canonicalize().map_err(|e| e.to_string())?
        {
            return Err("Task belongs to another workspace or an unsupported schema".into());
        }
        Ok(session)
    }
}
