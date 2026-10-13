use clap::Parser;
use cxt_core::{
    cli::Args,
    engine,
    review::{TaskSession, Workspace},
};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let name = format!(
            "cxt-review-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(name);
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn write(&self, name: &str, text: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn args(&self, flags: &[&str]) -> Args {
        Args::parse_from(std::iter::once("cxt").chain(flags.iter().copied())).normalized()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn preview_and_export_share_transformation_and_detect_source_changes() {
    let f = Fixture::new();
    let secret = "x".repeat(24);
    f.write(
        "src/main.rs",
        &format!(
            "// comment\nconst TOKEN: &str = \"{}\";\nfn main() {{}}\n",
            secret
        ),
    );
    let args = f.args(&["--minify", "--secrets", "redact", "--manifest"]);
    let workspace = Workspace::load(&args, &f.0).unwrap();
    let selected = workspace
        .files
        .iter()
        .map(|s| s.file.path.clone())
        .collect();
    let plan = workspace.plan(&args, &f.0, &selected, &BTreeSet::new());
    let result = plan.result.unwrap();
    assert_eq!(
        workspace.files[0].file.content,
        result.packed_files[0].content
    );
    assert!(!result.rendered.contains(&secret));
    assert!(!result.packed_files[0].content.contains("// comment"));
    assert!(workspace.stale_paths(&selected).is_empty());
    f.write("src/main.rs", "fn main() { println!(\"changed\"); }\n");
    assert_eq!(workspace.stale_paths(&selected).len(), 1);
}

#[test]
fn fail_policy_sanitizes_preview_and_blocks_plan() {
    let f = Fixture::new();
    let secret = "q".repeat(28);
    f.write(
        "main.rs",
        &format!("const PASSWORD: &str = \"{}\";", secret),
    );
    let args = f.args(&["--secrets", "fail"]);
    let w = Workspace::load(&args, &f.0).unwrap();
    assert!(!w.files[0].file.content.contains(&secret));
    let selected = w.files.iter().map(|s| s.file.path.clone()).collect();
    let plan = w.plan(&args, &f.0, &selected, &BTreeSet::new());
    assert!(plan.result.unwrap_err().contains("secret policy 'fail'"));
}

#[test]
fn checks_identify_missing_imports_and_reverse_relationships() {
    let f = Fixture::new();
    f.write(
        "app.ts",
        "import {answer} from './helper';\nconsole.log(answer);\n",
    );
    f.write("helper.ts", "export const answer = 42;\n");
    let args = f.args(&[]);
    let w = Workspace::load(&args, &f.0).unwrap();
    let app = f.0.join("app.ts").canonicalize().unwrap();
    let helper = f.0.join("helper.ts").canonicalize().unwrap();
    let selected = BTreeSet::from([app]);
    let plan = w.plan(&args, &f.0, &selected, &BTreeSet::new());
    assert!(plan
        .checks
        .iter()
        .any(|s| s.contains("Missing dependency:") && s.contains("helper.ts")));
    assert!(w
        .relationships(&helper)
        .iter()
        .any(|s| s.contains("← app.ts")));
}

#[test]
fn pinning_prioritizes_essential_files_and_reports_budget_exclusions() {
    let f = Fixture::new();
    f.write("a-noise.txt", &"noise ".repeat(100));
    f.write("z-essential.rs", "fn answer() -> u8 { 42 }\n");
    let args = f.args(&["--budget", "100"]);
    let w = Workspace::load(&args, &f.0).unwrap();
    let selected = w.files.iter().map(|s| s.file.path.clone()).collect();
    let essential = f.0.join("z-essential.rs").canonicalize().unwrap();
    let plan = w.plan(&args, &f.0, &selected, &BTreeSet::from([essential.clone()]));
    let result = plan.result.unwrap();
    assert!(result.estimated_tokens <= 100);
    assert_eq!(result.packed_files.len(), 1);
    assert_eq!(result.packed_files[0].path, essential);
    assert_eq!(plan.excluded.len(), 1);
    assert!(plan.checks.iter().any(|s| s.contains("a-noise.txt")));
}

#[test]
fn metadata_budget_does_not_silently_export_an_empty_context() {
    let f = Fixture::new();
    f.write("main.rs", "fn main() {}\n");
    let args = f.args(&["--budget", "15"]);
    assert!(engine::run(&args, &f.0)
        .unwrap_err()
        .to_string()
        .contains("overhead"));
}

#[test]
fn saved_task_roundtrips_options_without_source_contents_or_overwriting_files() {
    let f = Fixture::new();
    f.write("main.rs", "fn proprietary_implementation() {}\n");
    let args = f.args(&[
        "--budget", "500", "--format", "json", "--minify", "--output", "out.json",
    ]);
    let w = Workspace::load(&args, &f.0).unwrap();
    let selected = w.files.iter().map(|s| s.file.path.clone()).collect();
    let session = TaskSession::capture(&w, &args, &selected, &selected);
    let path = f.0.join("task.json");
    session.save(&path).unwrap();
    let loaded = TaskSession::load(&path, &w.prepared.root).unwrap();
    assert_eq!(loaded.args.max_tokens, Some(500));
    assert!(loaded.args.minify);
    assert_eq!(loaded.selected.len(), 1);
    assert!(!fs::read_to_string(&path)
        .unwrap()
        .contains("proprietary_implementation"));
    assert!(session.save(&path).is_err());
    let other = Fixture::new();
    assert!(TaskSession::load(&path, &other.0).is_err());
}

#[test]
fn saved_task_from_a_subdirectory_keeps_paths_and_output_location() {
    let f = Fixture::new();
    assert!(std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&f.0)
        .status()
        .unwrap()
        .success());
    f.write("src/main.rs", "fn main() {}\n");
    let cwd = f.0.join("src");
    let args = f.args(&["main.rs", "--output", "out.md"]);
    let w = Workspace::load(&args, &cwd).unwrap();
    assert_eq!(
        w.relative(&w.files[0].file.path).replace('\\', "/"),
        "src/main.rs"
    );
    let selected = w.files.iter().map(|s| s.file.path.clone()).collect();
    let task = TaskSession::capture(&w, &args, &selected, &selected);
    let path = f.0.join("task.json");
    task.save(&path).unwrap();
    let loaded = TaskSession::load(&path, &f.0).unwrap();
    assert!(loaded.args.paths[0].is_absolute());
    assert_eq!(
        loaded.args.output.as_ref().unwrap(),
        &cwd.canonicalize().unwrap().join("out.md")
    );
    assert_eq!(
        engine::run(&loaded.args, &f.0).unwrap().packed_files.len(),
        1
    );
}

#[test]
fn git_candidates_patches_graph_and_pins_share_canonical_file_identity() {
    let f = Fixture::new();
    assert!(std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&f.0)
        .status()
        .unwrap()
        .success());
    f.write(
        "app.ts",
        "import { answer } from './helper';\nconsole.log(answer);\n",
    );
    f.write("helper.ts", "export const answer = 42;\n");
    let args = f.args(&["--git", "working"]);
    let w = Workspace::load(&args, &f.0).unwrap();
    let app = f.0.join("app.ts").canonicalize().unwrap();
    let helper = f.0.join("helper.ts").canonicalize().unwrap();
    assert_eq!(w.root, f.0.canonicalize().unwrap());
    assert!(w.files.iter().any(|s| s.file.path == app));
    assert!(w.changes.iter().any(|c| c.path == app));
    assert!(w.dependencies[&app].contains(&helper));
    assert_eq!(w.relative(&app), "app.ts");
    let selected = BTreeSet::from([app.clone()]);
    let plan = w.plan(&args, &f.0, &selected, &selected);
    assert_eq!(plan.result.unwrap().packed_files[0].path, app);
    assert!(plan
        .checks
        .iter()
        .any(|s| s.contains("Missing dependency:") && s.contains("helper.ts")));
}

#[test]
fn rust_trace_handles_library_imports_grouped_uses_and_nested_modules() {
    let f = Fixture::new();
    f.write(
        "Cargo.toml",
        "[package]\nname = 'demo'\nversion = '0.1.0'\n[lib]\nname = 'demo_core'\n",
    );
    f.write(
        "src/main.rs",
        "use demo_core::engine::{self, run};\nfn main() { run(); }\n",
    );
    f.write("src/lib.rs", "pub mod engine;\n");
    f.write("src/engine.rs", "mod worker;\nuse self::worker::run as work;\nuse crate::engine::worker::{self, run};\nuse external::not_local;\npub fn run() { work(); }\n");
    f.write("src/engine/worker.rs", "use super::run;\npub fn run() {}\n");
    f.write(
        "src/external.rs",
        "// must not be treated as a local dependency\n",
    );
    let args = f.args(&["src/main.rs", "--trace"]);
    let result = engine::run(&args, &f.0).unwrap();
    let names: BTreeSet<_> = result
        .packed_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        BTreeSet::from([
            "main.rs".into(),
            "lib.rs".into(),
            "engine.rs".into(),
            "worker.rs".into()
        ])
    );
}

#[test]
fn rust_dependencies_resolve_path_attributes_and_parent_symbols() {
    let f = Fixture::new();
    f.write(
        "Cargo.toml",
        "[package]\nname = 'demo'\nversion = '0.1.0'\n[lib]\npath = 'src/core.rs'\n",
    );
    f.write("src/core.rs", "pub mod engine;\n");
    f.write(
        "src/engine.rs",
        "#[path = \"helper.rs\"]\nmod helper;\nmod worker;\npub fn parent() {}\n",
    );
    f.write("src/helper.rs", "pub fn help() {}\n");
    f.write("src/engine/worker.rs", "use super::parent;\n");
    let engine = f.0.join("src/engine.rs").canonicalize().unwrap();
    let helper = f.0.join("src/helper.rs").canonicalize().unwrap();
    let worker = f.0.join("src/engine/worker.rs").canonicalize().unwrap();
    assert!(cxt_core::tracer::direct_dependencies(&engine).contains(&helper));
    assert!(cxt_core::tracer::direct_dependencies(&worker).contains(&engine));
    let args = f.args(&["src/core.rs", "--trace"]);
    let result = engine::run(&args, &f.0).unwrap();
    assert_eq!(result.packed_files.len(), 4);
}
