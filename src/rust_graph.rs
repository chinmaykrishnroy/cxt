use crate::resolver::find_project_root;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

struct CrateContext<'a> {
    dir: &'a Path,
    name: &'a str,
    lib: &'a Path,
}

pub(crate) fn dependencies(file: &Path, node: Node, source: &[u8]) -> Vec<PathBuf> {
    let root = find_project_root(file.parent().unwrap_or(Path::new(".")));
    let manifest = fs::read_to_string(root.join("Cargo.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Value>().ok());
    let lib = manifest.as_ref().and_then(|m| m.get("lib"));
    let lib_path = root.join(
        lib.and_then(|l| l.get("path"))
            .and_then(|p| p.as_str())
            .unwrap_or("src/lib.rs"),
    );
    let crate_name = lib
        .and_then(|l| l.get("name"))
        .and_then(|n| n.as_str())
        .or_else(|| manifest.as_ref()?.get("package")?.get("name")?.as_str())
        .unwrap_or("")
        .replace('-', "_");
    let crate_dir = if lib_path.is_file() {
        lib_path.parent().unwrap_or(&root)
    } else {
        file.parent().unwrap_or(&root)
    };
    let parent = file.parent().unwrap_or(&root);
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let module_dir = if matches!(stem, "lib" | "main" | "mod")
        || file.canonicalize().ok() == lib_path.canonicalize().ok() && lib_path.is_file()
    {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    };
    let mut output = BTreeSet::new();
    walk(
        node,
        source,
        &module_dir,
        parent,
        &CrateContext {
            dir: crate_dir,
            name: &crate_name,
            lib: &lib_path,
        },
        &mut output,
    );
    output.remove(file);
    output.into_iter().collect()
}

fn walk(
    node: Node,
    source: &[u8],
    module_dir: &Path,
    attribute_dir: &Path,
    krate: &CrateContext<'_>,
    out: &mut BTreeSet<PathBuf>,
) {
    if node.kind() == "mod_item" {
        if let Some(name) = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
        {
            if let Some(body) = node.child_by_field_name("body") {
                walk(
                    body,
                    source,
                    &module_dir.join(name),
                    &module_dir.join(name),
                    krate,
                    out,
                );
                return;
            }
            // #[path = "..."] belongs to the following module declaration.
            let override_path = node
                .prev_named_sibling()
                .filter(|n| n.kind() == "attribute_item")
                .and_then(|n| n.utf8_text(source).ok())
                .filter(|text| text.contains("path"))
                .and_then(|text| text.split('"').nth(1));
            let path = override_path
                .map(|p| attribute_dir.join(p))
                .filter(|p| p.is_file())
                .or_else(|| module_file(&module_dir.join(name)));
            if let Some(path) = path {
                out.insert(path);
            }
            return;
        }
    }
    if node.kind() == "use_declaration" {
        if let Some(argument) = node
            .child_by_field_name("argument")
            .and_then(|n| n.utf8_text(source).ok())
        {
            for import in use_paths(argument) {
                let mut parts: Vec<&str> = import.split("::").filter(|p| !p.is_empty()).collect();
                let Some(first) = parts.first().copied() else {
                    continue;
                };
                let mut base = match first {
                    "crate" => krate.dir.to_path_buf(),
                    "self" | "super" => module_dir.to_path_buf(),
                    name if !krate.name.is_empty() && name == krate.name => {
                        if krate.lib.is_file() {
                            out.insert(krate.lib.to_path_buf());
                        }
                        krate.dir.to_path_buf()
                    }
                    _ => continue, // External crates must never resolve to similarly named local files.
                };
                if first == "super" {
                    while parts.first() == Some(&"super") {
                        base.pop();
                        parts.remove(0);
                    }
                } else {
                    parts.remove(0);
                }
                if parts.last() == Some(&"self") {
                    parts.pop();
                }
                // The final segments can name symbols rather than modules.
                let mut resolved = false;
                while !parts.is_empty() {
                    let path = parts
                        .iter()
                        .fold(base.clone(), |p, segment| p.join(segment));
                    if let Some(path) = module_file(&path) {
                        out.insert(path);
                        resolved = true;
                        break;
                    }
                    parts.pop();
                }
                if !resolved {
                    if base == krate.dir && krate.lib.is_file() {
                        out.insert(krate.lib.to_path_buf());
                    } else if let Some(path) = module_file(&base) {
                        out.insert(path);
                    }
                }
            }
        }
        return;
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            walk(child, source, module_dir, attribute_dir, krate, out);
        }
    }
}

fn module_file(path: &Path) -> Option<PathBuf> {
    let flat = path.with_extension("rs");
    if flat.is_file() {
        Some(flat)
    } else {
        let nested = path.join("mod.rs");
        nested.is_file().then_some(nested)
    }
}

fn use_paths(text: &str) -> Vec<String> {
    if let Some(open) = text.find('{') {
        let prefix = &text[..open];
        let inner = &text[open + 1..text.rfind('}').unwrap_or(text.len())];
        let mut depth = 0;
        let mut start = 0;
        let mut paths = Vec::new();
        for (i, c) in inner.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    paths.extend(use_paths(&format!("{}{}", prefix, inner[start..i].trim())));
                    start = i + 1;
                }
                _ => {}
            }
        }
        paths.extend(use_paths(&format!("{}{}", prefix, inner[start..].trim())));
        paths
    } else {
        vec![text
            .split(" as ")
            .next()
            .unwrap_or(text)
            .trim()
            .replace(' ', "")]
    }
}
