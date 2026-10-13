use crate::{cli::Args, config::Config, filter, tracer};
use ignore::WalkBuilder;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ScanLimits {
    pub max_entries: usize,
    pub max_bytes: u64,
    pub max_depth: usize,
    pub timeout: Duration,
    pub allow_broad: bool,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_entries: 20_000,
            max_bytes: 100 * 1024 * 1024,
            max_depth: 32,
            timeout: Duration::from_secs(15),
            allow_broad: false,
        }
    }
}

impl ScanLimits {
    pub fn for_args(args: &Args, cwd: &Path) -> Self {
        let config = Config::load(cwd);
        let defaults = Self::default();
        Self {
            max_entries: args
                .scan_max_entries
                .or(config.scan_max_entries)
                .unwrap_or(defaults.max_entries),
            max_bytes: args
                .scan_max_mb
                .or(config.scan_max_mb)
                .unwrap_or(defaults.max_bytes / (1024 * 1024))
                .saturating_mul(1024 * 1024),
            max_depth: args
                .scan_max_depth
                .or(config.scan_max_depth)
                .unwrap_or(defaults.max_depth),
            timeout: Duration::from_secs(
                args.scan_timeout
                    .or(config.scan_timeout)
                    .unwrap_or(defaults.timeout.as_secs()),
            ),
            allow_broad: args.allow_broad_scan,
        }
    }

    pub fn guard(&self, path: &Path) -> Result<(), String> {
        if is_broad_directory(path) && !self.allow_broad {
            return Err(format!(
                "WARNING: refusing to scan broad directory {} (home directory, its ancestor, or filesystem root). Choose a project folder, or explicitly pass --allow-broad-scan. Scan limits still apply; in the TUI enable 'Allow broad scan' in Settings.",
                path.display()
            ));
        }
        Ok(())
    }
}

pub fn is_broad_directory(path: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let home = directories::BaseDirs::new().map(|d| {
        d.home_dir()
            .canonicalize()
            .unwrap_or_else(|_| d.home_dir().to_path_buf())
    });
    broad_relative_to_home(&path, home.as_deref())
}

fn broad_relative_to_home(path: &Path, home: Option<&Path>) -> bool {
    path.parent().is_none() || home.is_some_and(|home| home.starts_with(path))
}

pub struct ScanBudget {
    limits: ScanLimits,
    started: Instant,
    entries: usize,
    bytes: u64,
    files: BTreeSet<PathBuf>,
}

impl ScanBudget {
    pub fn new(limits: ScanLimits) -> Self {
        Self {
            limits,
            started: Instant::now(),
            entries: 0,
            bytes: 0,
            files: BTreeSet::new(),
        }
    }

    pub fn check_time(&self) -> Result<(), String> {
        if !self.limits.timeout.is_zero() && self.started.elapsed() > self.limits.timeout {
            return Err("Scan stopped: time limit reached. Narrow the input or increase --scan-timeout (0 disables this limit). No partial context was exported.".into());
        }
        Ok(())
    }

    pub fn visit(&mut self, depth: usize) -> Result<(), String> {
        self.check_time()?;
        self.entries = self.entries.saturating_add(1);
        if self.limits.max_entries > 0 && self.entries > self.limits.max_entries {
            return Err(format!("Scan stopped: {} entry limit reached. Narrow the input or increase --scan-max-entries (0 disables this limit). No partial context was exported.", self.limits.max_entries));
        }
        if self.limits.max_depth > 0 && depth > self.limits.max_depth {
            return Err(format!("Scan stopped: depth limit {} reached. Narrow the input or increase --scan-max-depth (0 disables this limit). No partial context was exported.", self.limits.max_depth));
        }
        Ok(())
    }

    pub fn account_file(&mut self, path: &Path) -> Result<(), String> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self.files.insert(path.clone()) {
            self.check_time()?;
            self.account_bytes(fs::metadata(path).map_or(0, |m| m.len()))?;
        }
        Ok(())
    }

    pub fn account_bytes(&mut self, bytes: u64) -> Result<(), String> {
        self.check_time()?;
        self.bytes = self.bytes.saturating_add(bytes);
        if self.limits.max_bytes > 0 && self.bytes > self.limits.max_bytes {
            return Err(format!("Scan stopped: {} MiB input limit reached. Narrow the input or increase --scan-max-mb (0 disables this limit). No partial context was exported.", self.limits.max_bytes / (1024 * 1024)));
        }
        Ok(())
    }

    pub fn read_file(&mut self, path: &Path) -> Result<(), String> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !self.files.contains(&canonical) {
            self.visit(0)?;
        }
        self.account_file(path)
    }
}

pub(crate) struct Files {
    pub paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

pub(crate) fn walk_files(
    root: &Path,
    args: &Args,
    cwd: &Path,
    graph: bool,
) -> Result<Files, String> {
    let limits = ScanLimits::for_args(args, cwd);
    limits.guard(root)?;
    let depth = limits.max_depth;
    let mut budget = ScanBudget::new(limits);
    let no_limit = args.no_limit && !graph;
    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(args.exclude_hidden && !graph)
        .follow_links(false)
        .skip_stdout(true)
        .sort_by_file_name(|a, b| a.cmp(b));
    if depth > 0 {
        // Inspect one extra level to report truncation rather than silently omit it.
        walker.max_depth(Some(depth.saturating_add(1)));
    }
    walker.filter_entry(move |entry| {
        no_limit
            || !entry.file_type().is_some_and(|t| t.is_dir())
            || !filter::is_junk_path(entry.path())
    });
    let mut result = Files {
        paths: Vec::new(),
        warnings: Vec::new(),
    };
    for entry in walker.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                budget.visit(0)?;
                result.warnings.push(error.to_string());
                continue;
            }
        };
        budget.visit(entry.depth())?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.into_path();
        if !no_limit
            && (filter::is_junk_file(&path)
                || fs::metadata(&path).is_ok_and(|m| m.len() > 1024 * 1024))
        {
            if !graph {
                result.paths.push(path);
            }
            continue;
        }
        if graph
            && !path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| tracer::get_language(s).is_some())
        {
            continue;
        }
        budget.account_file(&path)?;
        result.paths.push(path);
    }
    budget.check_time()?;
    Ok(result)
}

pub(crate) struct Graph {
    pub dependencies: BTreeMap<PathBuf, Vec<PathBuf>>,
    pub warnings: Vec<String>,
}

pub(crate) fn graph(root: &Path, args: &Args, cwd: &Path) -> Result<Graph, String> {
    let files = walk_files(root, args, cwd, true)?;
    let mut budget = ScanBudget::new(ScanLimits::for_args(args, cwd));
    let mut dependencies = BTreeMap::new();
    for path in files.paths {
        budget.read_file(&path)?;
        let path = path.canonicalize().unwrap_or(path);
        dependencies.insert(path.clone(), tracer::direct_dependencies(&path));
    }
    budget.check_time()?;
    Ok(Graph {
        dependencies,
        warnings: files.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broad_guard_distinguishes_home_and_ancestors_from_projects() {
        let home = std::env::temp_dir().join("cxt-home-fixture");
        assert!(broad_relative_to_home(&home, Some(&home)));
        assert!(broad_relative_to_home(home.parent().unwrap(), Some(&home)));
        assert!(!broad_relative_to_home(&home.join("project"), Some(&home)));
        assert!(!broad_relative_to_home(
            &home.with_file_name("another-user"),
            Some(&home)
        ));
        assert!(broad_relative_to_home(
            home.ancestors().last().unwrap(),
            None
        ));
    }

    #[test]
    fn timeout_is_cooperative_and_zero_disables_it() {
        let mut budget = ScanBudget::new(ScanLimits {
            timeout: Duration::from_secs(1),
            ..ScanLimits::default()
        });
        budget.started = Instant::now() - Duration::from_secs(2);
        assert!(budget.check_time().unwrap_err().contains("time limit"));
        budget.limits.timeout = Duration::ZERO;
        assert!(budget.check_time().is_ok());
    }
}
