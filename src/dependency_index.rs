use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SCHEMA: &str = "cxt.dependencies.v1";
const MAX_ENTRY: usize = 4 * 1024 * 1024;
const MAX_OBSERVATIONS: usize = 512;
const MAX_VALIDATION_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default, Debug, Clone, Serialize)]
pub struct GraphStats {
    pub reused: usize,
    pub rebuilt: usize,
    pub uncacheable: usize,
}

pub struct DependencyIndex {
    directory: Option<PathBuf>,
}
pub struct Resolution {
    pub configuration_inputs: Vec<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub cache_hit: bool,
    pub cacheable: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
enum State {
    Probe {
        canonical: Option<PathBuf>,
        kind: u8,
    },
    File {
        canonical: PathBuf,
        hash: String,
    },
    Directory {
        canonical: PathBuf,
        entries: Vec<(PathBuf, u8)>,
    },
}
#[derive(Clone, Serialize, Deserialize)]
struct Observation {
    path: PathBuf,
    state: State,
}
#[derive(Serialize, Deserialize)]
struct Entry {
    schema: String,
    source_hash: String,
    paths: Vec<PathBuf>,
    observations: Vec<Observation>,
}
#[derive(Default)]
struct Ledger {
    configurations: BTreeSet<PathBuf>,
    observations: BTreeMap<(PathBuf, u8), Observation>,
    disabled: bool,
}

// Resolution is synchronous. Per-thread, scoped ledgers keep filesystem observations
// out of every resolver signature; nested captures feed their enclosing capture too.
thread_local! { static LEDGERS: RefCell<Vec<Ledger>> = const { RefCell::new(Vec::new()) }; }
struct Capture;
impl Capture {
    fn start() -> Self {
        LEDGERS.with(|l| l.borrow_mut().push(Ledger::default()));
        Self
    }
    fn finish(self) -> Ledger {
        let ledger = LEDGERS.with(|l| l.borrow_mut().pop().unwrap_or_default());
        std::mem::forget(self);
        ledger
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        LEDGERS.with(|l| {
            l.borrow_mut().pop();
        });
    }
}
fn active() -> bool {
    LEDGERS.with(|l| !l.borrow().is_empty())
}
fn record(path: &Path, state: State) {
    LEDGERS.with(|ledgers| {
        for ledger in ledgers.borrow_mut().iter_mut() {
            if let State::File { canonical, .. } = &state {
                if ledger.configurations.len() <= 2048 {
                    ledger.configurations.insert(canonical.clone());
                }
            }
            if ledger.observations.len() >= MAX_OBSERVATIONS {
                ledger.disabled = true;
            }
            if !ledger.disabled {
                let tag = match &state {
                    State::Probe { .. } => 0,
                    State::File { .. } => 1,
                    State::Directory { .. } => 2,
                };
                ledger.observations.insert(
                    (path.into(), tag),
                    Observation {
                        path: path.into(),
                        state: state.clone(),
                    },
                );
            }
        }
    });
}
pub(crate) fn uncacheable() {
    LEDGERS.with(|l| {
        for ledger in l.borrow_mut().iter_mut() {
            ledger.disabled = true;
        }
    });
}
fn kind(path: &Path) -> u8 {
    match fs::metadata(path) {
        Ok(m) if m.is_file() => 1,
        Ok(m) if m.is_dir() => 2,
        Ok(_) => 3,
        Err(_) => 0,
    }
}
fn probe(path: &Path) -> State {
    State::Probe {
        canonical: path.canonicalize().ok(),
        kind: kind(path),
    }
}
fn observe_probe(path: &Path) {
    if active() {
        record(path, probe(path));
    }
}

pub(crate) trait ObservedPath: AsRef<Path> {
    fn observed_is_file(&self) -> bool {
        observe_probe(self.as_ref());
        self.as_ref().is_file()
    }
    fn observed_is_dir(&self) -> bool {
        observe_probe(self.as_ref());
        self.as_ref().is_dir()
    }
    fn observed_exists(&self) -> bool {
        observe_probe(self.as_ref());
        self.as_ref().exists()
    }
    fn observed_canonicalize(&self) -> std::io::Result<PathBuf> {
        observe_probe(self.as_ref());
        self.as_ref().canonicalize()
    }
}
impl<T: AsRef<Path>> ObservedPath for T {}

pub(crate) fn read_text(path: &Path, limit: usize) -> std::io::Result<String> {
    let result = (|| {
        let mut text = String::new();
        fs::File::open(path)?
            .take((limit as u64).saturating_add(1))
            .read_to_string(&mut text)?;
        if text.len() > limit {
            return Err(std::io::Error::other("configuration exceeds read limit"));
        }
        Ok(text)
    })();
    if active() {
        match &result {
            Ok(text) if text.len() <= MAX_ENTRY => record(
                path,
                State::File {
                    canonical: path.canonicalize()?,
                    hash: digest(text.as_bytes()),
                },
            ),
            _ => uncacheable(),
        }
    }
    result
}

pub(crate) fn directory(path: &Path, limit: usize) -> std::io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path)?
        .take(limit.saturating_add(1))
        .collect::<Result<Vec<_>, _>>()?;
    if entries.len() > limit {
        uncacheable();
        return Err(std::io::Error::other("directory entry limit exceeded"));
    }
    let mut snapshot = Vec::new();
    let mut paths = Vec::new();
    for entry in entries {
        let t = entry.file_type()?;
        snapshot.push((
            entry.file_name().into(),
            if t.is_symlink() {
                3
            } else if t.is_dir() {
                2
            } else {
                1
            },
        ));
        paths.push(entry.path());
    }
    snapshot.sort();
    paths.sort();
    if active() {
        record(
            path,
            State::Directory {
                canonical: path.canonicalize()?,
                entries: snapshot,
            },
        );
    }
    Ok(paths)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

impl DependencyIndex {
    pub fn new(directory: Option<PathBuf>) -> Self {
        Self { directory }
    }
    pub fn for_args(args: &crate::cli::Args, cwd: &Path) -> Self {
        let directory = if args.no_cache || args.mcp {
            None
        } else {
            args.cache_dir
                .as_ref()
                .map(|p| {
                    if p.is_absolute() {
                        p.clone()
                    } else {
                        cwd.join(p)
                    }
                })
                .or_else(|| {
                    directories::ProjectDirs::from("", "", "cxt")
                        .map(|d| d.cache_dir().join("symbols"))
                })
                .map(|p| p.join("dependencies"))
        };
        Self::new(directory)
    }

    pub fn resolve(&self, file: &Path, scope: Option<&Path>) -> Result<Resolution, String> {
        let file = crate::scope::check(scope, file)?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let Ok(source) = fs::read_to_string(&file) else {
            // Preserve the graph's best-effort handling of unreadable/non-text source.
            // Packing independently reports skipped inputs and never trusts this as content.
            return Ok(Resolution {
                configuration_inputs: Vec::new(),
                paths: Vec::new(),
                cache_hit: false,
                cacheable: false,
            });
        };
        let source_hash = digest(source.as_bytes());
        let scope = scope
            .map(|p| p.canonicalize())
            .transpose()
            .map_err(|e| e.to_string())?;
        let key = digest(
            format!(
                "{SCHEMA}:{}:{}:{:?}",
                env!("CARGO_PKG_VERSION"),
                file.display(),
                scope
            )
            .as_bytes(),
        );
        let cache = self
            .directory
            .as_ref()
            .map(|p| p.join(format!("{key}.json")));
        if let Some(cache) = &cache {
            if let Ok(f) = fs::File::open(cache) {
                let mut bytes = Vec::new();
                if f.take((MAX_ENTRY + 1) as u64)
                    .read_to_end(&mut bytes)
                    .is_ok()
                    && bytes.len() <= MAX_ENTRY
                {
                    if let Ok(entry) = serde_json::from_slice::<Entry>(&bytes) {
                        if entry.schema == SCHEMA
                            && entry.source_hash == source_hash
                            && entry.observations.len() <= MAX_OBSERVATIONS
                            && entry.paths.len() <= 20_000
                            && entry
                                .paths
                                .iter()
                                .all(|p| crate::scope::check(scope.as_deref(), p).is_ok())
                            && valid(&entry.observations, scope.as_deref())
                        {
                            return Ok(Resolution {
                                configuration_inputs: entry
                                    .observations
                                    .iter()
                                    .filter_map(|o| match &o.state {
                                        State::File { canonical, .. } => Some(canonical.clone()),
                                        _ => None,
                                    })
                                    .collect(),
                                paths: entry.paths,
                                cache_hit: true,
                                cacheable: true,
                            });
                        }
                    }
                }
            }
        }
        let capture = Capture::start();
        let paths = crate::tracer::dependencies_from_source(&file, &source, scope.as_deref())?;
        let ledger = capture.finish();
        if ledger.configurations.len() > 2048 {
            return Err(
                "dependency resolver exceeded 2048 configuration inputs; no partial graph returned"
                    .into(),
            );
        }
        let configuration_inputs = ledger.configurations.into_iter().collect();
        let cacheable = self.directory.is_some() && !ledger.disabled && source.len() <= MAX_ENTRY;
        if cacheable {
            if let Some(cache) = cache {
                if let Some(parent) = cache.parent() {
                    let entry = Entry {
                        schema: SCHEMA.into(),
                        source_hash,
                        paths: paths.clone(),
                        observations: ledger.observations.into_values().collect(),
                    };
                    if let Ok(bytes) = serde_json::to_vec(&entry) {
                        if bytes.len() <= MAX_ENTRY && fs::create_dir_all(parent).is_ok() {
                            if let Ok(mut temp) = tempfile::NamedTempFile::new_in(parent) {
                                if temp.write_all(&bytes).is_ok() {
                                    let _ = temp.persist(cache);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(Resolution {
            configuration_inputs,
            paths,
            cache_hit: false,
            cacheable,
        })
    }
}

fn valid(observations: &[Observation], scope: Option<&Path>) -> bool {
    let started = Instant::now();
    let mut bytes = 0usize;
    for observation in observations {
        if started.elapsed() > Duration::from_secs(2) {
            return false;
        }
        let current = match &observation.state {
            State::Probe { .. } => probe(&observation.path),
            State::File { .. } => {
                if crate::scope::check(scope, &observation.path).is_err() {
                    return false;
                }
                let Ok(text) = read_text(&observation.path, MAX_ENTRY) else {
                    return false;
                };
                bytes = bytes.saturating_add(text.len());
                if bytes > MAX_VALIDATION_BYTES {
                    return false;
                }
                let Ok(canonical) = observation.path.canonicalize() else {
                    return false;
                };
                State::File {
                    canonical,
                    hash: digest(text.as_bytes()),
                }
            }
            State::Directory { entries, .. } => {
                if crate::scope::check(scope, &observation.path).is_err() {
                    return false;
                }
                let Ok(paths) = directory(&observation.path, entries.len()) else {
                    return false;
                };
                let mut snapshot = Vec::new();
                for path in paths {
                    let Ok(m) = fs::symlink_metadata(&path) else {
                        return false;
                    };
                    snapshot.push((
                        path.file_name().unwrap_or_default().into(),
                        if m.is_symlink() {
                            3
                        } else if m.is_dir() {
                            2
                        } else {
                            1
                        },
                    ));
                }
                snapshot.sort();
                let Ok(canonical) = observation.path.canonicalize() else {
                    return false;
                };
                State::Directory {
                    canonical,
                    entries: snapshot,
                }
            }
        };
        // State contains only paths, kind tags and digests; no source bodies.
        if current != observation.state {
            return false;
        }
    }
    true
}
