use crate::tracer::{direct_dependencies, get_language};
use ignore::WalkBuilder;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

pub fn expand(root: &Path, seeds: &[PathBuf]) -> Vec<(PathBuf, String)> {
    let mut source_files = Vec::new();
    let mut tests = Vec::new();

    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .skip_stdout(true);
    for entry in walker.build().flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.into_path();
        if junk_dir(&path, root) {
            continue;
        }
        let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
        if get_language(ext).is_some() {
            if looks_like_test(&path) {
                tests.push(path.clone());
            }
            source_files.push(path);
        }
    }

    let mut reverse: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for file in &source_files {
        let file_canon = file.canonicalize().unwrap_or_else(|_| file.clone());
        for dependency in direct_dependencies(file) {
            let dep = dependency.canonicalize().unwrap_or(dependency);
            reverse.entry(dep).or_default().push(file_canon.clone());
        }
    }

    let seed_set: BTreeSet<PathBuf> = seeds
        .iter()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
        .collect();
    let mut seen = seed_set.clone();
    let mut queue: VecDeque<(PathBuf, usize)> = seed_set.iter().cloned().map(|p| (p, 0)).collect();
    let mut output = Vec::new();

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= 2 {
            continue;
        }
        if let Some(users) = reverse.get(&current) {
            for user in users {
                if seen.insert(user.clone()) {
                    output.push((
                        user.clone(),
                        format!("reverse-dependency:{}", display_rel(root, &current)),
                    ));
                    queue.push_back((user.clone(), depth + 1));
                }
            }
        }
    }

    let seed_stems: Vec<String> = seeds
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
        .map(normalize_stem)
        .filter(|s| !s.is_empty())
        .collect();
    for test in tests {
        let test_stem = normalize_stem(test.file_stem().and_then(|s| s.to_str()).unwrap_or(""));
        if seed_stems
            .iter()
            .any(|seed| test_stem.contains(seed) || seed.contains(&test_stem))
        {
            let canon = test.canonicalize().unwrap_or(test);
            if seen.insert(canon.clone()) {
                output.push((canon, "related-test".to_string()));
            }
        }
    }

    output.sort_by(|a, b| a.0.cmp(&b.0));
    output
}

pub(crate) fn looks_like_test(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("\\test\\")
        || lower.contains("\\tests\\")
        || lower.contains("__tests__")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_test.go")
        || lower.ends_with("_test.py")
        || path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("test_"))
}

pub(crate) fn normalize_stem(stem: &str) -> String {
    stem.to_ascii_lowercase()
        .trim_start_matches("test_")
        .to_string()
        .replace(".test", "")
        .replace(".spec", "")
        .replace("_test", "")
        .replace("-test", "")
        .replace(['_', '-'], "")
}

fn junk_dir(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .any(|c| {
            matches!(
                c.as_os_str().to_string_lossy().as_ref(),
                ".git" | "target" | "node_modules" | "vendor" | ".venv" | "dist" | "build"
            )
        })
}

fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
