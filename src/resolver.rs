use crate::dependency_index::ObservedPath;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};

fn export_targets(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(a) => a.iter().flat_map(export_targets).collect(),
        serde_json::Value::Object(map) => ["types", "import", "default", "require", "browser"]
            .iter()
            .filter_map(|k| map.get(*k))
            .flat_map(export_targets)
            .collect(),
        _ => Vec::new(),
    }
}

fn workspace_package(
    base: &Path,
    import: &str,
    scope: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    for root in base.ancestors().take(16) {
        if crate::scope::check(scope, root).is_err() {
            break;
        }
        if crate::scan::ScanLimits::default().guard(root).is_err() {
            break;
        }
        let Some(text) = scoped_text(scope, &root.join("package.json")) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let patterns = manifest.get("workspaces").and_then(|v| {
            v.as_array()
                .or_else(|| v.get("packages").and_then(|x| x.as_array()))
        });
        let Some(patterns) = patterns else {
            continue;
        };
        // Git/global ignore state is external to this resolver's observations.
        crate::dependency_index::uncacheable();
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in patterns.iter().filter_map(|p| p.as_str()) {
            if let Ok(glob) = globset::Glob::new(pattern) {
                builder.add(glob);
            }
        }
        let Ok(globs) = builder.build() else {
            continue;
        };
        for (visited, entry) in ignore::WalkBuilder::new(root)
            .follow_links(false)
            .build()
            .enumerate()
        {
            if visited >= 20_000 {
                return Err(
                    "Node workspace resolver exceeded 20000 entries; narrow the workspace".into(),
                );
            }
            let entry =
                entry.map_err(|_| "Node workspace resolver could not read a directory entry")?;
            if entry.depth() > 8 {
                return Err(
                    "Node workspace resolver exceeded depth 8; narrow the workspace".into(),
                );
            }
            if entry.file_name() != "package.json" {
                continue;
            }
            let Some(dir) = entry.path().parent() else {
                continue;
            };
            if !globs.is_match(dir.strip_prefix(root).unwrap_or(dir)) {
                continue;
            }
            let Some(text) = scoped_text(scope, entry.path()) else {
                continue;
            };
            let Ok(package) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            let Some(name) = package.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let suffix = if import == name {
                ""
            } else if let Some(s) = import.strip_prefix(&format!("{name}/")) {
                s
            } else {
                continue;
            };
            let key = if suffix.is_empty() {
                ".".into()
            } else {
                format!("./{suffix}")
            };
            let mut targets = Vec::new();
            if let Some(exports) = package.get("exports") {
                if let Some(value) = exports.get(&key) {
                    targets.extend(export_targets(value));
                } else if key == "." {
                    targets.extend(export_targets(exports));
                }
                if let Some(map) = exports.as_object() {
                    for (pattern, value) in map {
                        for target in export_targets(value) {
                            if let Some(mapped) = apply_tsconfig_pattern(&key, pattern, &target) {
                                targets.push(mapped);
                            }
                        }
                    }
                }
            } else if suffix.is_empty() {
                targets.extend(
                    ["types", "module", "main"].iter().filter_map(|k| {
                        package.get(*k).and_then(|v| v.as_str()).map(str::to_string)
                    }),
                );
                targets.extend(["src/index", "index"].map(str::to_string));
            } else {
                targets.push(suffix.into());
            }
            for target in targets {
                if let Some(path) = resolve_as_file_or_dir(&dir.join(target), Some("ts")) {
                    return Ok(Some(path));
                }
            }
        }
    }
    Ok(None)
}

#[derive(Deserialize, Debug, Default, Clone)]
struct TsConfig {
    extends: Option<serde_json::Value>,
    #[serde(skip)]
    paths_origin: PathBuf,
    #[serde(rename = "compilerOptions")]
    compiler_options: Option<CompilerOptions>,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct CompilerOptions {
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    paths: Option<HashMap<String, Vec<String>>>,
}

pub fn find_project_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    loop {
        if current.join("tsconfig.json").observed_is_file()
            || current.join("package.json").observed_is_file()
            || current.join("pyproject.toml").observed_is_file()
            || current.join("setup.py").observed_is_file()
            || current.join("Cargo.toml").observed_is_file()
            || current.join("go.mod").observed_is_file()
            || current.join("go.work").observed_is_file()
            || current.join("pom.xml").observed_is_file()
            || current.join("build.gradle").observed_is_file()
            || current.join(".git").observed_exists()
        {
            return current;
        }
        if !current.pop() {
            break;
        }
    }
    start.to_path_buf()
}

fn find_tsconfig(start: &Path, scope: Option<&Path>) -> Option<(PathBuf, TsConfig)> {
    let mut current = start.to_path_buf();
    loop {
        for name in &["tsconfig.json", "tsconfig.base.json", "jsconfig.json"] {
            let path = current.join(name);
            if path.observed_is_file() {
                if let Some(tsconfig) = load_tsconfig(&path, &mut HashSet::new(), 0, scope) {
                    return Some((current.clone(), tsconfig));
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn bounded_text(path: &Path) -> Option<String> {
    crate::dependency_index::read_text(path, 512 * 1024).ok()
}

pub(crate) fn scoped_text(scope: Option<&Path>, path: &Path) -> Option<String> {
    path.observed_exists();
    let path = crate::scope::check(scope, path).ok()?;
    bounded_text(&path)
}

fn load_tsconfig(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    scope: Option<&Path>,
) -> Option<TsConfig> {
    crate::scope::check(scope, path).ok()?;
    let path = path.observed_canonicalize().ok()?;
    if depth > 16 || !visited.insert(path.clone()) {
        return None;
    }
    let dir = path.parent()?;
    let child: TsConfig = json5::from_str(&bounded_text(&path)?).ok()?;
    let parents = match &child.extends {
        Some(serde_json::Value::String(s)) => vec![s.clone()],
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    };
    let mut merged = TsConfig {
        paths_origin: dir.into(),
        ..Default::default()
    };
    for parent in parents {
        let mut candidate = dir.join(&parent);
        if !parent.starts_with('.') && !Path::new(&parent).is_absolute() {
            for ancestor in dir.ancestors() {
                let package = ancestor.join("node_modules").join(&parent);
                if package.observed_is_file() || package.with_extension("json").observed_is_file() {
                    candidate = package;
                    break;
                }
                if package.join("tsconfig.json").observed_is_file() {
                    candidate = package.join("tsconfig.json");
                    break;
                }
            }
        }
        if !candidate.observed_is_file() {
            candidate = candidate.with_extension("json");
        }
        merged = load_tsconfig(&candidate, visited, depth + 1, scope)?;
    }
    let mut options = merged.compiler_options.take().unwrap_or_default();
    if let Some(child_options) = child.compiler_options {
        if let Some(base) = child_options.base_url {
            options.base_url = Some(dir.join(base).to_string_lossy().into_owned());
        }
        if child_options.paths.is_some() {
            options.paths = child_options.paths;
            merged.paths_origin = dir.into();
        }
    }
    merged.compiler_options = Some(options);
    visited.remove(&path);
    Some(merged)
}

fn apply_tsconfig_pattern(import: &str, pattern: &str, target: &str) -> Option<String> {
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() != 2 {
            return None;
        }
        let prefix = parts[0];
        let suffix = parts[1];
        if import.starts_with(prefix)
            && import.ends_with(suffix)
            && import.len() >= prefix.len() + suffix.len()
        {
            let wildcard = &import[prefix.len()..import.len() - suffix.len()];
            return Some(target.replace('*', wildcard));
        }
    } else if import == pattern {
        return Some(target.to_string());
    }
    None
}

fn resolve_tsconfig_alias(
    tsconfig_dir: &Path,
    import: &str,
    tsconfig: &TsConfig,
) -> Option<PathBuf> {
    let compiler_options = tsconfig.compiler_options.as_ref()?;
    let paths = compiler_options.paths.as_ref()?;

    let base_url_dir = match &compiler_options.base_url {
        Some(base) => tsconfig_dir.join(base),
        None => {
            if tsconfig.paths_origin.as_os_str().is_empty() {
                tsconfig_dir.to_path_buf()
            } else {
                tsconfig.paths_origin.clone()
            }
        }
    };

    let mut patterns: Vec<(String, Vec<String>)> =
        paths.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    patterns.sort_by_key(|(pattern, _)| std::cmp::Reverse(pattern.len()));

    for (pattern, targets) in patterns {
        if targets.is_empty() {
            continue;
        }
        for target in targets {
            if let Some(resolved) = apply_tsconfig_pattern(import, &pattern, &target) {
                let candidate = base_url_dir.join(resolved);
                if let Some(p) = resolve_as_file_or_dir(&candidate, None) {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn extension_candidates(extension: &str) -> &'static [&'static str] {
    match extension {
        "py" => &["py"],
        "go" => &["go"],
        "java" => &["java"],
        "cs" => &["cs"],
        "rb" => &["rb"],
        "php" => &["php"],
        "rs" => &["rs"],
        "mts" | "cts" => &["mts", "cts", "ts", "tsx"],
        "c" | "h" => &["c", "h"],
        "cpp" | "hpp" | "cc" | "cxx" | "ino" => &["cpp", "hpp", "cc", "cxx", "h", "ino"],
        _ => &["ts", "tsx", "js", "jsx"], // Default TS/JS
    }
}

fn index_files(extension: &str) -> &'static [&'static str] {
    if extension == "py" {
        &["__init__.py"]
    } else {
        &["index.ts", "index.tsx", "index.js", "index.jsx"]
    }
}

fn resolve_as_file_or_dir(path: &Path, extension: Option<&str>) -> Option<PathBuf> {
    if path.observed_is_file() {
        return Some(path.to_path_buf());
    }

    let ext = extension.unwrap_or("");
    if matches!(ext, "ts" | "tsx" | "mts" | "cts")
        && matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("js" | "mjs" | "cjs")
        )
    {
        for substitute in ["ts", "tsx", "mts", "cts"] {
            let candidate = path.with_extension(substitute);
            if candidate.observed_is_file() {
                return Some(candidate);
            }
        }
    }
    let path_str = path.to_string_lossy();

    for candidate in extension_candidates(ext) {
        let with_ext = PathBuf::from(format!("{}.{}", path_str, candidate));
        if with_ext.observed_is_file() {
            return Some(with_ext);
        }
    }

    if path.observed_is_dir() {
        if let Some(dir_name) = path.file_name() {
            let dir_name = dir_name.to_string_lossy();
            for candidate in extension_candidates(ext) {
                let named_file = path.join(format!("{}.{}", dir_name, candidate));
                if named_file.observed_is_file() {
                    return Some(named_file);
                }
            }
        }

        for index in index_files(ext) {
            let index_path = path.join(index);
            if index_path.observed_is_file() {
                return Some(index_path);
            }
        }
    }

    None
}

pub fn resolve_import_path(
    base_dir: &Path,
    import_path: &str,
    current_lang: &str,
) -> Option<PathBuf> {
    resolve_import_paths_scoped(base_dir, import_path, current_lang, None)
        .into_iter()
        .next()
}

fn resolve_import_path_scoped(
    base_dir: &Path,
    import_path: &str,
    current_lang: &str,
    scope: Option<&Path>,
) -> Option<PathBuf> {
    // Relative imports always start with ./ or ../
    if import_path.starts_with("./") || import_path.starts_with("../") {
        // Normalize "./foo" to "foo" so the path join is clean.
        let cleaned = if let Some(stripped) = import_path.strip_prefix("./") {
            stripped
        } else {
            import_path
        };
        let candidate = base_dir.join(cleaned);
        return resolve_as_file_or_dir(&candidate, Some(current_lang));
    }

    // C/C++ quoted includes (e.g. #include "header.h" or #include "dir/header.h")
    // are relative to the current source file, even without a leading ./.
    if matches!(
        current_lang,
        "c" | "cpp" | "h" | "hpp" | "hh" | "hxx" | "cc" | "cxx" | "ino"
    ) {
        let candidate = base_dir.join(import_path);
        if let Some(p) = resolve_as_file_or_dir(&candidate, Some(current_lang)) {
            return Some(p);
        }
        return None;
    }

    // TypeScript/JS path aliases via tsconfig/jsconfig.
    if matches!(
        current_lang,
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
    ) {
        if let Some((tsconfig_dir, tsconfig)) = find_tsconfig(base_dir, scope) {
            if let Some(resolved) = resolve_tsconfig_alias(&tsconfig_dir, import_path, &tsconfig) {
                return Some(resolved);
            }
        }
    }

    // Python package imports: turn dots into path separators and search
    // relative to the current file, then the project root (and a src/ layout).
    if current_lang == "py" {
        if import_path.chars().all(|c| c == '.') {
            return None;
        }
        let leading_dots = import_path.chars().take_while(|&c| c == '.').count();
        let after_dots = &import_path[leading_dots..];

        let normalized = after_dots.replace('.', "/");
        if leading_dots > 0 {
            let mut relative = base_dir.to_path_buf();
            for _ in 1..leading_dots {
                relative.pop();
            }
            return resolve_as_file_or_dir(&relative.join(&normalized), Some("py"));
        }

        let project_root = find_project_root(base_dir);

        let candidate = base_dir.join(&normalized);
        if let Some(p) = resolve_as_file_or_dir(&candidate, Some("py")) {
            return Some(p);
        }

        let candidate = project_root.join(&normalized);
        if let Some(p) = resolve_as_file_or_dir(&candidate, Some("py")) {
            return Some(p);
        }

        let src_dir = project_root.join("src");
        if src_dir.observed_is_dir() {
            let candidate = src_dir.join(&normalized);
            if let Some(p) = resolve_as_file_or_dir(&candidate, Some("py")) {
                return Some(p);
            }
        }

        return None;
    }

    // Generic module resolution for dotted/slash import paths (Java, C#, Go, PHP, etc.).
    // Convert dots to path separators and look from the project root (and a src/ layout).
    let project_root = find_project_root(base_dir);
    let mut normalized = import_path.replace(['.', '\\'], "/");
    if current_lang == "go" {
        if let Some(module) = scoped_text(scope, &project_root.join("go.mod")).and_then(|t| {
            t.lines().find_map(|l| {
                l.trim()
                    .strip_prefix("module ")
                    .map(|m| m.trim().trim_matches('"').to_string())
            })
        }) {
            if let Some(relative) = import_path.strip_prefix(&format!("{module}/")) {
                normalized = relative.into();
            }
        }
    }

    let candidate = project_root.join(&normalized);
    if let Some(p) = resolve_as_file_or_dir(&candidate, Some(current_lang)) {
        return Some(p);
    }

    let src_dir = project_root.join("src");
    if src_dir.observed_is_dir() {
        let candidate = src_dir.join(&normalized);
        if let Some(p) = resolve_as_file_or_dir(&candidate, Some(current_lang)) {
            return Some(p);
        }
    }

    if current_lang == "java" {
        for directory in ["src/main/java", "src/test/java"] {
            let candidate = project_root.join(directory).join(&normalized);
            if let Some(path) = resolve_as_file_or_dir(&candidate, Some("java")) {
                return Some(path);
            }
        }
    }

    // Bare JS/TS imports (e.g. "react") are treated as external / node_modules.
    None
}

pub fn resolve_import_paths(base: &Path, import: &str, language: &str) -> Vec<PathBuf> {
    resolve_import_paths_scoped(base, import, language, None)
}

pub fn resolve_import_paths_scoped(
    base: &Path,
    import: &str,
    language: &str,
    scope: Option<&Path>,
) -> Vec<PathBuf> {
    resolve_import_paths_scoped_checked(base, import, language, scope).unwrap_or_default()
}

pub(crate) fn resolve_import_paths_scoped_checked(
    base: &Path,
    import: &str,
    language: &str,
    scope: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    if matches!(
        language,
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
    ) && !import.starts_with('.')
    {
        if let Some((dir, config)) = find_tsconfig(base, scope) {
            if let Some(path) = resolve_tsconfig_alias(&dir, import, &config) {
                return Ok(vec![path]);
            }
        }
        if let Some(path) = workspace_package(base, import, scope)? {
            return Ok(vec![path]);
        }
    }
    if language == "go" {
        for (module, root) in go_modules(base, scope)? {
            let relative = if import == module {
                Some("")
            } else {
                import.strip_prefix(&format!("{module}/"))
            };
            if let Some(relative) = relative {
                let directory = root.join(relative);
                if crate::scope::check(scope, &directory).is_err() {
                    return Ok(Vec::new());
                }
                if directory.observed_is_dir() {
                    let entries = crate::dependency_index::directory(&directory, 1000).map_err(|_| "Go package resolver exceeded limits or could not read a directory entry")?;
                    let mut files: Vec<_> = entries
                        .into_iter()
                        .filter(|p| {
                            p.extension().is_some_and(|e| e == "go")
                                && !p
                                    .file_name()
                                    .is_some_and(|n| n.to_string_lossy().ends_with("_test.go"))
                        })
                        .collect();
                    files.sort();
                    return Ok(files);
                }
            }
        }
    }
    Ok(resolve_import_path_scoped(base, import, language, scope)
        .into_iter()
        .collect())
}

fn go_modules(base: &Path, scope: Option<&Path>) -> Result<Vec<(String, PathBuf)>, String> {
    let roots = go_roots(base, scope)?;
    let mut modules = std::collections::BTreeMap::new();
    let mut configurations = Vec::new();
    for root in roots {
        if let Some(text) = scoped_text(scope, &root.join("go.mod")) {
            if let Some(module) = text.lines().find_map(|l| {
                l.trim()
                    .strip_prefix("module ")
                    .map(|s| s.trim().trim_matches('"').to_string())
            }) {
                modules.insert(module, root.clone());
            }
            configurations.push((root, text));
        }
    }
    for ancestor in base.ancestors().take(16) {
        if crate::scope::check(scope, ancestor).is_err()
            || crate::scan::ScanLimits::default().guard(ancestor).is_err()
        {
            break;
        }
        if let Some(work) = scoped_text(scope, &ancestor.join("go.work")) {
            configurations.push((ancestor.to_path_buf(), work));
            break;
        }
    }
    for (origin, text) in configurations {
        let mut block = false;
        for (line_number, line) in text.lines().enumerate() {
            if line_number >= 1024 {
                return Err("Go resolver exceeded 1024 configuration lines".into());
            }
            let line = line.split("//").next().unwrap_or("").trim();
            if line == "replace (" {
                block = true;
                continue;
            }
            if line == ")" {
                block = false;
                continue;
            }
            let directive = if block {
                Some(line)
            } else {
                line.strip_prefix("replace ")
            };
            let Some((left, right)) = directive.and_then(|s| s.split_once("=>")) else {
                continue;
            };
            let Some(name) = left.split_whitespace().next() else {
                continue;
            };
            let target = right.trim().trim_matches('"');
            if !target.starts_with('.') && !Path::new(target).is_absolute() {
                continue;
            }
            let target = origin.join(target);
            if crate::scope::check(scope, &target).is_ok() {
                modules.insert(name.trim_matches('"').into(), target);
            }
            if modules.len() > 128 {
                return Err("Go resolver exceeded 128 module mappings".into());
            }
        }
    }
    let mut modules: Vec<_> = modules.into_iter().collect();
    modules.sort_by(|(a, _), (b, _)| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    Ok(modules)
}

fn go_roots(base: &Path, scope: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    let mut roots = vec![find_project_root(base)];
    for ancestor in base.ancestors().take(16) {
        if crate::scope::check(scope, ancestor).is_err()
            || crate::scan::ScanLimits::default().guard(ancestor).is_err()
        {
            break;
        }
        let Some(work) = scoped_text(scope, &ancestor.join("go.work")) else {
            continue;
        };
        let mut use_block = false;
        for (number, line) in work.lines().enumerate() {
            if number >= 1024 {
                return Err("Go workspace resolver exceeded 1024 configuration lines".into());
            }
            let line = line.split("//").next().unwrap_or("").trim();
            if line == "use (" {
                use_block = true;
                continue;
            }
            if line == ")" {
                use_block = false;
                continue;
            }
            let entry = if use_block {
                Some(line)
            } else {
                line.strip_prefix("use ")
            };
            if let Some(entry) = entry.filter(|s| !s.is_empty()) {
                let root = ancestor.join(entry.trim_matches('"'));
                if crate::scope::check(scope, &root).is_ok() && !roots.contains(&root) {
                    if roots.len() >= 128 {
                        return Err("Go workspace resolver exceeded 128 module roots".into());
                    }
                    roots.push(root);
                }
            }
        }
        break;
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("cxt-resolver-{}-{}-{}", prefix, pid, nanos));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn resolve_relative_python_module() {
        let dir = temp_dir("py-rel");
        write(&dir, "helper.py", "def run(): pass\n");

        let resolved = resolve_import_path(&dir, "./helper", "py");
        assert!(resolved.is_some());
        assert!(resolved.as_ref().unwrap().file_name().unwrap() == "helper.py");
    }

    #[test]
    fn resolve_dotted_python_package() {
        let dir = temp_dir("py-dotted");
        write(&dir, "pkg/__init__.py", "");

        let resolved = resolve_import_path(&dir, "pkg", "py");
        assert!(resolved.is_some());
        assert!(resolved
            .as_ref()
            .unwrap()
            .ends_with(Path::new("pkg/__init__.py")));
    }

    #[test]
    fn resolve_c_quoted_include() {
        let dir = temp_dir("c-include");
        write(&dir, "helper.h", "#pragma once\n");

        let resolved = resolve_import_path(&dir, "helper.h", "c");
        assert!(resolved.is_some());
        assert!(resolved.as_ref().unwrap().file_name().unwrap() == "helper.h");
    }

    #[test]
    fn resolve_c_subdir_include() {
        let dir = temp_dir("c-subdir");
        write(&dir, "lib/helper.h", "#pragma once\n");

        let resolved = resolve_import_path(&dir, "lib/helper.h", "c");
        assert!(resolved.is_some());
        assert!(resolved
            .as_ref()
            .unwrap()
            .ends_with(Path::new("lib/helper.h")));
    }

    #[test]
    fn resolve_java_dotted_import() {
        let dir = temp_dir("java-dotted");
        write(&dir, "com/example/Helper.java", "package com.example;\n");

        let resolved = resolve_import_path(&dir, "com.example.Helper", "java");
        assert!(resolved.is_some());
        assert!(resolved.as_ref().unwrap().ends_with("Helper.java"));
    }

    #[test]
    fn resolve_external_module_returns_none() {
        let dir = temp_dir("external");

        assert!(resolve_import_path(&dir, "react", "js").is_none());
        assert!(resolve_import_path(&dir, "os", "py").is_none());
        assert!(resolve_import_path(&dir, "java.util.List", "java").is_none());
    }
}
