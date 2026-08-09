use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug, Default)]
struct TsConfig {
    #[serde(rename = "compilerOptions")]
    compiler_options: Option<CompilerOptions>,
}

#[derive(Deserialize, Debug, Default)]
struct CompilerOptions {
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    paths: Option<HashMap<String, Vec<String>>>,
}

pub fn find_project_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    loop {
        if current.join("tsconfig.json").is_file()
            || current.join("package.json").is_file()
            || current.join("pyproject.toml").is_file()
            || current.join("setup.py").is_file()
            || current.join("Cargo.toml").is_file()
            || current.join(".git").is_dir()
        {
            return current;
        }
        if !current.pop() {
            break;
        }
    }
    start.to_path_buf()
}

fn find_tsconfig(start: &Path) -> Option<(PathBuf, TsConfig)> {
    let mut current = start.to_path_buf();
    loop {
        for name in &["tsconfig.json", "tsconfig.base.json", "jsconfig.json"] {
            let path = current.join(name);
            if path.is_file() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(tsconfig) = serde_json::from_str::<TsConfig>(&content) {
                        return Some((current.clone(), tsconfig));
                    }
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
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
        None => tsconfig_dir.to_path_buf(),
    };

    let mut patterns: Vec<(String, Vec<String>)> =
        paths.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    patterns.sort_by_key(|(pattern, _)| std::cmp::Reverse(pattern.len()));

    for (pattern, targets) in patterns {
        if targets.is_empty() {
            continue;
        }
        let target = &targets[0];
        if let Some(resolved) = apply_tsconfig_pattern(import, &pattern, target) {
            let candidate = base_url_dir.join(resolved);
            if let Some(p) = resolve_as_file_or_dir(&candidate, None) {
                return Some(p);
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
        "c" | "h" => &["c", "h"],
        "cpp" | "hpp" | "cc" | "cxx" => &["cpp", "hpp", "cc", "cxx"],
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
    if path.is_file() {
        return Some(path.to_path_buf());
    }

    let ext = extension.unwrap_or("");
    let path_str = path.to_string_lossy();

    for candidate in extension_candidates(ext) {
        let with_ext = PathBuf::from(format!("{}.{}", path_str, candidate));
        if with_ext.is_file() {
            return Some(with_ext);
        }
    }

    if path.is_dir() {
        if let Some(dir_name) = path.file_name() {
            let dir_name = dir_name.to_string_lossy();
            for candidate in extension_candidates(ext) {
                let named_file = path.join(format!("{}.{}", dir_name, candidate));
                if named_file.is_file() {
                    return Some(named_file);
                }
            }
        }

        for index in index_files(ext) {
            let index_path = path.join(index);
            if index_path.is_file() {
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
    if matches!(current_lang, "c" | "cpp" | "h" | "hpp" | "cc" | "cxx") {
        let candidate = base_dir.join(import_path);
        if let Some(p) = resolve_as_file_or_dir(&candidate, Some(current_lang)) {
            return Some(p);
        }
        return None;
    }

    // TypeScript/JS path aliases via tsconfig/jsconfig.
    if current_lang != "py" {
        if let Some((tsconfig_dir, tsconfig)) = find_tsconfig(base_dir) {
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

        let mut prefix = String::new();
        for _ in 1..leading_dots {
            prefix.push_str("../");
        }
        let path_part = if prefix.is_empty() {
            after_dots.to_string()
        } else {
            format!("{}{}", prefix, after_dots)
        };
        let normalized = path_part.replace('.', "/");

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
        if src_dir.is_dir() {
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
    let normalized = import_path.replace(['.', '\\'], "/");

    let candidate = project_root.join(&normalized);
    if let Some(p) = resolve_as_file_or_dir(&candidate, Some(current_lang)) {
        return Some(p);
    }

    let src_dir = project_root.join("src");
    if src_dir.is_dir() {
        let candidate = src_dir.join(&normalized);
        if let Some(p) = resolve_as_file_or_dir(&candidate, Some(current_lang)) {
            return Some(p);
        }
    }

    // Bare JS/TS imports (e.g. "react") are treated as external / node_modules.
    None
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
