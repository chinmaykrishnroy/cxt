use crate::resolver::resolve_import_path;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Node, Parser as TsParser};

pub fn get_language(extension: &str) -> Option<Language> {
    match extension {
        "py" => Some(tree_sitter_python::language()),
        "js" | "jsx" => Some(tree_sitter_javascript::language()),
        "ts" => Some(tree_sitter_typescript::language_typescript()),
        "tsx" => Some(tree_sitter_typescript::language_tsx()),
        "rs" => Some(tree_sitter_rust::language()),
        "go" => Some(tree_sitter_go::language()),
        "c" | "h" => Some(tree_sitter_c::language()),
        "cpp" | "cc" | "cxx" | "hpp" => Some(tree_sitter_cpp::language()),
        "java" => Some(tree_sitter_java::language()),
        "cs" => Some(tree_sitter_c_sharp::language()),
        "rb" => Some(tree_sitter_ruby::language()),
        "php" => Some(tree_sitter_php::language()),
        _ => None,
    }
}

fn clean_import_text(text: &str) -> String {
    text.trim_matches(|c| c == '\'' || c == '"' || c == '`' || c == '<' || c == '>')
        .to_string()
}

fn find_first_string(node: Node, source: &[u8]) -> Option<String> {
    let kind = node.kind();
    if kind == "string"
        || kind == "string_literal"
        || kind == "interpreted_string_literal"
        || kind == "raw_string_literal"
        || kind == "system_lib_string"
        || kind == "encapsed_string"
    {
        if let Ok(text) = std::str::from_utf8(&source[node.byte_range()]) {
            return Some(clean_import_text(text));
        }
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Some(s) = find_first_string(child, source) {
                return Some(s);
            }
        }
    }
    None
}

fn extract_declared_module(node: Node, source: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(&source[node.byte_range()]).ok()?.trim();
    if let Some(start) = text.find('"').or_else(|| text.find('\'')) {
        let quote = text.as_bytes()[start] as char;
        let rest = &text[start + 1..];
        return rest.find(quote).map(|end| rest[..end].to_string());
    }
    let text = text
        .trim_start_matches("import")
        .trim_start_matches("using")
        .trim()
        .trim_end_matches(';')
        .trim();
    let module = text.split_whitespace().next()?.trim_end_matches(';');
    (!module.is_empty()).then(|| module.to_string())
}

fn collect_dotted_names(node: Node, source: &[u8], imports: &mut Vec<String>) {
    if node.kind() == "dotted_name"
        || node.kind() == "scoped_identifier"
        || node.kind() == "qualified_name"
        || node.kind() == "member_access_expression"
        || node.kind() == "identifier"
    {
        if let Ok(text) = std::str::from_utf8(&source[node.byte_range()]) {
            imports.push(text.to_string());
        }
        return;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_dotted_names(child, source, imports);
        }
    }
}

fn collect_imported_names(node: Node, source: &[u8], names: &mut Vec<String>) {
    if node.kind() == "dotted_name" || node.kind() == "identifier" {
        if let Ok(text) = std::str::from_utf8(&source[node.byte_range()]) {
            names.push(text.to_string());
        }
        return;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_imported_names(child, source, names);
        }
    }
}

fn extract_imports_from_node(node: Node, source: &[u8], imports: &mut Vec<String>) {
    let kind = node.kind();

    // Catch C/C++ `#include "..."`
    if kind == "preproc_include" {
        // C/C++ standard libraries use `< >`. We only want to pack local project files, which use `" "`.
        let raw_text = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
        if raw_text.contains('<') && raw_text.contains('>') {
            return; // Skip system libraries to save filesystem lookups
        }

        if let Some(string_node) = find_first_string(node, source) {
            imports.push(string_node);
        }
        return;
    }

    // Catch Go, Java, and C# imports/using statements.
    if kind == "import_declaration" || kind == "import_spec" || kind == "using_directive" {
        if let Some(module) = extract_declared_module(node, source) {
            imports.push(module);
        }
        return;
    }

    if kind == "import_statement" {
        // JS/TS imports always carry a string source (e.g. import x from './foo').
        // Python imports use dotted_name modules (e.g. import os.path).
        let mut has_string_source = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "string" {
                    if let Ok(text) = std::str::from_utf8(&source[child.byte_range()]) {
                        imports.push(clean_import_text(text));
                        has_string_source = true;
                    }
                }
            }
        }
        if has_string_source {
            return;
        }
        // Python: collect every dotted_name (module path) inside this statement.
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                collect_dotted_names(child, source, imports);
            }
        }
        return;
    }

    if kind == "import_from_statement" {
        // Python: from <module> import <names>
        // The module is everything between the leading 'from' and the 'import' keyword.
        let mut import_keyword_seen = false;
        let mut prefix = String::new();
        let mut imported_names = Vec::new();
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "import" {
                    let range = node.byte_range();
                    let module_src = &source[range.start..child.start_byte()];
                    if let Ok(text) = std::str::from_utf8(module_src) {
                        if let Some(module) = text.trim().strip_prefix("from") {
                            prefix = module.trim().to_string();
                        }
                    }
                    import_keyword_seen = true;
                    continue;
                }
                if import_keyword_seen {
                    collect_imported_names(child, source, &mut imported_names);
                }
            }
        }

        if !prefix.is_empty() && !prefix.chars().all(|c| c == '.') {
            imports.push(prefix.clone());
        }

        // For 'from . import a, b' the names themselves are modules relative to the package.
        if prefix.chars().all(|c| c == '.') && !imported_names.is_empty() {
            for name in imported_names {
                if name == "*" {
                    continue;
                }
                imports.push(format!("{}{}", prefix, name));
            }
        }
        return;
    }

    // PHP include / require statements.
    if kind == "include_expression"
        || kind == "include_once_expression"
        || kind == "require_expression"
        || kind == "require_once_expression"
    {
        if let Some(string_node) = find_first_string(node, source) {
            imports.push(string_node);
        }
        return;
    }

    if kind == "call_expression" || kind == "call" {
        // Catch CommonJS: require('...') and dynamic import: import('...')
        // Ruby uses `call` nodes for `require './foo'`.
        let mut function_name = "";
        let mut args_node: Option<Node> = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "identifier" | "import" => {
                        if let Ok(text) = std::str::from_utf8(&source[child.byte_range()]) {
                            function_name = text;
                        }
                    }
                    "arguments" | "argument_list" => args_node = Some(child),
                    _ => {}
                }
            }
        }
        if function_name == "require"
            || function_name == "import"
            || function_name == "require_relative"
        {
            if let Some(args) = args_node {
                if let Some(arg) = find_first_string(args, source) {
                    imports.push(arg);
                }
            }
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_imports_from_node(child, source, imports);
        }
    }
}

pub fn trace_dependencies<F>(entry_file: &Path, mut on_discover: F) -> Vec<PathBuf>
where
    F: FnMut(&Path),
{
    let mut queue = VecDeque::new();
    let mut collected = HashSet::new();

    let entry_abs = entry_file
        .canonicalize()
        .unwrap_or_else(|_| entry_file.to_path_buf());
    collected.insert(entry_abs.clone());
    queue.push_back(entry_abs);

    let mut parser = TsParser::new();

    while let Some(current_file) = queue.pop_front() {
        let ext = current_file
            .extension()
            .unwrap_or_default()
            .to_string_lossy();
        if let Some(lang) = get_language(&ext) {
            parser.set_language(lang).unwrap();

            if let Ok(content) = fs::read_to_string(&current_file) {
                if let Some(tree) = parser.parse(&content, None) {
                    let mut imports = Vec::new();
                    extract_imports_from_node(tree.root_node(), content.as_bytes(), &mut imports);

                    let base_dir = current_file.parent().unwrap_or(Path::new(""));
                    for imp in imports {
                        if let Some(resolved) = resolve_import_path(base_dir, &imp, &ext) {
                            let canonical = resolved.canonicalize().unwrap_or(resolved);
                            if collected.insert(canonical.clone()) {
                                on_discover(&canonical);
                                queue.push_back(canonical);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut result: Vec<PathBuf> = collected.into_iter().collect();
    result.sort();
    result
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
        let dir = std::env::temp_dir().join(format!("cxt-unit-{}-{}-{}", prefix, pid, nanos));
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
    fn clean_import_text_strips_quotes_and_angle_brackets() {
        assert_eq!(clean_import_text("'foo'"), "foo");
        assert_eq!(clean_import_text("\"bar\""), "bar");
        assert_eq!(clean_import_text("`<baz>`"), "baz");
        assert_eq!(clean_import_text("<stdio.h>"), "stdio.h");
    }

    #[test]
    fn get_language_returns_some_for_supported_extensions() {
        assert!(get_language("py").is_some());
        assert!(get_language("js").is_some());
        assert!(get_language("ts").is_some());
        assert!(get_language("rs").is_some());
        assert!(get_language("go").is_some());
        assert!(get_language("c").is_some());
        assert!(get_language("cpp").is_some());
        assert!(get_language("java").is_some());
        assert!(get_language("cs").is_some());
        assert!(get_language("rb").is_some());
        assert!(get_language("php").is_some());
        assert!(get_language("xyz").is_none());
    }

    #[test]
    fn trace_dependencies_python_module() {
        let dir = temp_dir("trace-py");
        write(&dir, "main.py", "import helper\nprint(helper.run())\n");
        write(&dir, "helper.py", "def run():\n    return 'hi'\n");

        let entry = dir.join("main.py").canonicalize().unwrap();
        let paths = trace_dependencies(&entry, |_| {});

        let names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"main.py".to_string()));
        assert!(names.contains(&"helper.py".to_string()));
    }

    #[test]
    fn trace_dependencies_skips_c_system_headers() {
        let dir = temp_dir("trace-c");
        write(
            &dir,
            "main.c",
            "#include <stdio.h>\n#include \"helper.h\"\nint main() { return 0; }\n",
        );
        write(&dir, "helper.h", "#pragma once\nvoid helper();\n");

        let entry = dir.join("main.c").canonicalize().unwrap();
        let paths = trace_dependencies(&entry, |_| {});

        let names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"main.c".to_string()));
        assert!(names.contains(&"helper.h".to_string()));
        assert!(
            !names.iter().any(|n| n.contains("stdio")),
            "system headers should not be traced"
        );
    }
}
