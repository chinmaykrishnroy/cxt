use crate::symbols::SourceIndex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "cxt.symbols.v6-field-separators-tree-sitter-0.20";
const MAX_CACHE_BYTES: u64 = 8 * 1024 * 1024;

pub struct IndexStore {
    directory: Option<PathBuf>,
}
pub struct IndexedSource {
    pub index: SourceIndex,
    pub cache_hit: bool,
}
#[derive(Serialize, Deserialize)]
struct Entry {
    schema: String,
    hash: String,
    index: SourceIndex,
}

impl IndexStore {
    pub fn new(directory: Option<PathBuf>) -> Self {
        Self { directory }
    }
    pub fn for_args(args: &crate::cli::Args, cwd: &Path) -> Self {
        let dir = if args.no_cache {
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
        };
        Self::new(dir)
    }
    pub fn load(&self, path: &Path, source: &str) -> IndexedSource {
        let hash = format!("{:x}", Sha256::digest(source.as_bytes()));
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let key = format!(
            "{:x}",
            Sha256::digest(
                format!(
                    "{SCHEMA}:{}:{}",
                    env!("CARGO_PKG_VERSION"),
                    canonical.display()
                )
                .as_bytes()
            )
        );
        let file = self
            .directory
            .as_ref()
            .map(|d| d.join(format!("{key}.json")));
        if let Some(file) = &file {
            if let Ok(f) = fs::File::open(file) {
                let mut bytes = Vec::new();
                if f.take(MAX_CACHE_BYTES + 1).read_to_end(&mut bytes).is_ok()
                    && bytes.len() as u64 <= MAX_CACHE_BYTES
                {
                    if let Ok(entry) = serde_json::from_slice::<Entry>(&bytes) {
                        let valid = entry
                            .index
                            .symbols
                            .iter()
                            .map(|s| &s.span)
                            .chain(entry.index.imports.iter())
                            .all(|s| {
                                s.start_byte <= s.end_byte
                                    && source.get(s.start_byte..s.end_byte).is_some()
                            });
                        if entry.schema == SCHEMA && entry.hash == hash && valid {
                            return IndexedSource {
                                index: entry.index,
                                cache_hit: true,
                            };
                        }
                    }
                }
            }
        }
        let index = crate::symbols::extract(path, source);
        if let Some(file) = file {
            if let Some(dir) = file.parent() {
                if fs::create_dir_all(dir).is_ok() {
                    if let Ok(mut temp) = tempfile::NamedTempFile::new_in(dir) {
                        let entry = Entry {
                            schema: SCHEMA.into(),
                            hash,
                            index: index.clone(),
                        };
                        if let Ok(bytes) = serde_json::to_vec(&entry) {
                            if bytes.len() as u64 <= MAX_CACHE_BYTES
                                && temp.write_all(&bytes).is_ok()
                            {
                                let _ = temp.persist(file);
                            }
                        }
                    }
                }
            }
        }
        IndexedSource {
            index,
            cache_hit: false,
        }
    }
}
