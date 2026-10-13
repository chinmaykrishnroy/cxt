use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Change {
    pub path: PathBuf,
    pub status: String,
    pub diff: String,
}

/// Review actual patches, including deletions. Packing still uses checkout files.
pub fn changes(root: &Path, mode: &str) -> Result<Vec<Change>, String> {
    let mut paths = std::collections::BTreeMap::new();
    let mut comparisons: Vec<Vec<&str>> = Vec::new();
    match mode {
        "working" | "diff" => {
            comparisons.push(vec![]);
            comparisons.push(vec!["--cached"]);
        }
        "staged" => comparisons.push(vec!["--cached"]),
        "unstaged" => comparisons.push(vec![]),
        "untracked" | "tracked" => {}
        range => comparisons.push(vec![range]),
    }
    for comparison in &comparisons {
        let mut args = vec!["diff", "--name-status", "--no-renames", "-z"];
        args.extend(comparison.iter().copied());
        args.push("--");
        let fields = run_paths(root, &args)?;
        for pair in fields.chunks_exact(2) {
            paths.insert(root.join(&pair[1]), pair[0].to_string_lossy().to_string());
        }
    }
    if matches!(mode, "working" | "diff" | "untracked") {
        for path in run_paths(root, &["ls-files", "--others", "--exclude-standard", "-z"])? {
            paths.insert(root.join(path), "??".to_string());
        }
    }
    let mut result = Vec::new();
    for (path, status) in paths {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let mut patches = Vec::new();
        if status == "??" {
            // Content is transformed and sanitized by the review module before display.
            patches.push(format!("Untracked: {}\n", relative.display()));
        } else {
            for comparison in &comparisons {
                let mut command = Command::new("git");
                command
                    .args([
                        "diff",
                        "--no-color",
                        "--no-ext-diff",
                        "--no-textconv",
                        "--no-renames",
                    ])
                    .args(comparison)
                    .arg("--")
                    .arg(relative)
                    .current_dir(root);
                let output = command.output().map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).into_owned());
                }
                if !output.stdout.is_empty() {
                    patches.push(String::from_utf8_lossy(&output.stdout).into_owned());
                }
            }
        }
        result.push(Change {
            path,
            status,
            diff: patches.join("\n"),
        });
    }
    Ok(result)
}

pub fn repository_root(start: &Path) -> Result<PathBuf, String> {
    let cwd = if start.is_file() {
        start.parent().unwrap_or(Path::new("."))
    } else {
        start
    };
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;
    if !output.status.success() {
        return Err("not inside a Git repository".to_string());
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let root = PathBuf::from(root);
    Ok(root.canonicalize().unwrap_or(root))
}

pub fn head_commit(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

pub fn select(root: &Path, mode: &str) -> Result<Vec<PathBuf>, String> {
    let normalized = mode.trim().to_ascii_lowercase();
    let mut files = BTreeSet::new();

    match normalized.as_str() {
        "working" | "diff" => {
            add(
                &mut files,
                run_paths(root, &["diff", "--name-only", "-z", "--"])?,
                root,
            );
            add(
                &mut files,
                run_paths(root, &["diff", "--cached", "--name-only", "-z", "--"])?,
                root,
            );
            add(
                &mut files,
                run_paths(root, &["ls-files", "--others", "--exclude-standard", "-z"])?,
                root,
            );
        }
        "staged" => add(
            &mut files,
            run_paths(root, &["diff", "--cached", "--name-only", "-z", "--"])?,
            root,
        ),
        "unstaged" => add(
            &mut files,
            run_paths(root, &["diff", "--name-only", "-z", "--"])?,
            root,
        ),
        "untracked" => add(
            &mut files,
            run_paths(root, &["ls-files", "--others", "--exclude-standard", "-z"])?,
            root,
        ),
        "tracked" => add(&mut files, run_paths(root, &["ls-files", "-z"])?, root),
        _ => {
            add(
                &mut files,
                run_paths(root, &["diff", "--name-only", "-z", mode, "--"])
                    .map_err(|e| format!("invalid Git mode/range '{}': {}", mode, e))?,
                root,
            );
        }
    }

    Ok(files.into_iter().collect())
}

fn add(set: &mut BTreeSet<PathBuf>, paths: Vec<PathBuf>, root: &Path) {
    for path in paths {
        let absolute = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        if absolute.is_file() {
            set.insert(absolute.canonicalize().unwrap_or(absolute));
        }
    }
}

fn run_paths(root: &Path, args: &[&str]) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git {:?} failed", args)
        } else {
            stderr
        });
    }
    Ok(output
        .stdout
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| PathBuf::from(String::from_utf8_lossy(part).to_string()))
        .collect())
}

/// Return Git porcelain status codes keyed by absolute file path.
/// Files outside a repository, clean files, and Git failures simply yield no entry.
pub fn status_map(root: &Path) -> std::collections::BTreeMap<PathBuf, String> {
    let mut map = std::collections::BTreeMap::new();
    let Ok(repo) = repository_root(root) else {
        return map;
    };
    let Ok(output) = Command::new("git")
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .current_dir(&repo)
        .output()
    else {
        return map;
    };
    if !output.status.success() {
        return map;
    }

    for record in output.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        if record.len() < 4 || record.get(2) != Some(&b' ') {
            continue;
        }
        let status = String::from_utf8_lossy(&record[..2]).to_string();
        let raw_path = String::from_utf8_lossy(&record[3..]).to_string();
        let absolute = repo.join(raw_path);
        let key = absolute.canonicalize().unwrap_or(absolute);
        map.insert(key, status);
    }
    map
}
