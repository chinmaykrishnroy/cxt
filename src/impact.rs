use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

pub struct ImpactResult {
    pub candidates: Vec<(PathBuf, String)>,
    pub warnings: Vec<String>,
}

pub fn expand(
    root: &Path,
    seeds: &[PathBuf],
    args: &crate::cli::Args,
    cwd: &Path,
) -> Result<ImpactResult, String> {
    let graph = crate::scan::graph(root, args, cwd)?;
    let tests: Vec<_> = graph
        .dependencies
        .keys()
        .filter(|p| looks_like_test(p))
        .cloned()
        .collect();

    let mut reverse: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for (file, dependencies) in &graph.dependencies {
        for dependency in dependencies {
            reverse
                .entry(dependency.clone())
                .or_default()
                .push(file.clone());
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
    Ok(ImpactResult {
        candidates: output,
        warnings: graph.warnings,
    })
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

fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
