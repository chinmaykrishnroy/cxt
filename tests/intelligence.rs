#[allow(dead_code)]
mod common;
use common::{assert_success, run_cxt, temp_dir, write_file};

#[test]
fn changing_resolver_configuration_impacts_unchanged_importers() {
    let dir = temp_dir("config-impact");
    common::init_git_repo(&dir);
    common::commit_file(
        &dir,
        "tsconfig.json",
        "{\"compilerOptions\":{\"baseUrl\":\".\",\"paths\":{\"@app/helper\":[\"helper.ts\"]}}}\n",
        "config",
    );
    common::commit_file(&dir, "helper.ts", "export const answer = 42;\n", "helper");
    common::commit_file(
        &dir,
        "main.ts",
        "import { answer } from '@app/helper'; console.log(answer);\n",
        "main",
    );
    write_file(&dir, "tsconfig.json", "{\"compilerOptions\":{\"strict\":true,\"baseUrl\":\".\",\"paths\":{\"@app/helper\":[\"helper.ts\"]}}}\n");
    let out = run_cxt(
        &dir,
        &[
            "--git", "working", "--impact", "--format", "json", "--stdout",
        ],
    );
    assert_success(&out);
    let packet: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(packet["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == "main.ts"));
    let uncached = run_cxt(
        &dir,
        &[
            "--git",
            "working",
            "--impact",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&uncached);
    let packet: serde_json::Value = serde_json::from_slice(&uncached.stdout).unwrap();
    assert!(packet["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == "main.ts"));
}

#[test]
fn cargo_target_specific_local_dependencies_are_discovered_conservatively() {
    let dir = temp_dir("cargo-target-deps");
    write_file(&dir, "app/Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n[target.'cfg(windows)'.dependencies]\nplatform = { path = '../platform' }\n");
    write_file(
        &dir,
        "app/src/main.rs",
        "use platform::answer;\nfn main() { answer(); }\n",
    );
    write_file(
        &dir,
        "platform/Cargo.toml",
        "[package]\nname='platform'\nversion='1.0.0'\n",
    );
    write_file(
        &dir,
        "platform/src/lib.rs",
        "pub fn answer() -> u8 { 42 }\n",
    );
    let out = run_cxt(
        &dir,
        &[
            "app/src/main.rs",
            "--trace",
            "--workspace-root",
            ".",
            "--stdout",
        ],
    );
    assert_success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("fn answer"));
}

#[test]
fn go_local_replacements_trace_the_replacement_package() {
    let dir = temp_dir("go-replace");
    write_file(
        &dir,
        "app/go.mod",
        "module example.local/app\nreplace example.local/shared => ../shared\n",
    );
    write_file(
        &dir,
        "app/main.go",
        "package main\nimport \"example.local/shared\"\nfunc main() { shared.Answer() }\n",
    );
    write_file(
        &dir,
        "shared/go.mod",
        "module example.local/implementation\n",
    );
    write_file(
        &dir,
        "shared/answer.go",
        "package shared\nfunc Answer() int { return 42 }\n",
    );
    let out = run_cxt(
        &dir,
        &[
            "app/main.go",
            "--trace",
            "--workspace-root",
            ".",
            "--stdout",
        ],
    );
    assert_success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("func Answer"));
}

#[test]
fn persistent_dependencies_reuse_edges_and_invalidate_configuration_and_missing_targets() {
    let dir = temp_dir("dependency-index");
    write_file(
        &dir,
        "tsconfig.json",
        "{\"compilerOptions\":{\"paths\":{\"@app/*\":[\"a/*\"]}}}",
    );
    write_file(&dir, "main.ts", "import { answer } from '@app/helper';\n");
    write_file(&dir, "a/helper.ts", "export const answer = 1;\n");
    write_file(&dir, "b/helper.ts", "export const answer = 2;\n");
    let index = cxt_core::dependency_index::DependencyIndex::new(Some(dir.join("cache")));
    let first = index.resolve(&dir.join("main.ts"), Some(&dir)).unwrap();
    assert!(!first.cache_hit);
    assert!(first.paths.iter().any(|p| p.ends_with("a/helper.ts")));
    assert!(
        index
            .resolve(&dir.join("main.ts"), Some(&dir))
            .unwrap()
            .cache_hit
    );
    write_file(
        &dir,
        "tsconfig.json",
        "{\"compilerOptions\":{\"paths\":{\"@app/*\":[\"b/*\"]}}}",
    );
    let changed = index.resolve(&dir.join("main.ts"), Some(&dir)).unwrap();
    assert!(!changed.cache_hit);
    assert!(changed.paths.iter().any(|p| p.ends_with("b/helper.ts")));
    write_file(&dir, "main.ts", "import { missing } from './later';\n");
    assert!(index
        .resolve(&dir.join("main.ts"), Some(&dir))
        .unwrap()
        .paths
        .is_empty());
    assert!(
        index
            .resolve(&dir.join("main.ts"), Some(&dir))
            .unwrap()
            .cache_hit
    );
    write_file(&dir, "later.ts", "export const missing = 42;\n");
    let added = index.resolve(&dir.join("main.ts"), Some(&dir)).unwrap();
    assert!(!added.cache_hit);
    assert!(added.paths.iter().any(|p| p.ends_with("later.ts")));
}

#[test]
fn persisted_graph_handles_deleted_renamed_branch_changed_and_corrupt_inputs() {
    let dir = temp_dir("dependency-mutations");
    write_file(&dir, "main.py", "import helper\n");
    write_file(&dir, "helper.py", "def answer(): return 42\n");
    let cache = dir.join("cache");
    let index = cxt_core::dependency_index::DependencyIndex::new(Some(cache.clone()));
    let main = dir.join("main.py");
    let first = index.resolve(&main, Some(&dir)).unwrap();
    assert_eq!(first.paths.len(), 1);
    assert!(index.resolve(&main, Some(&dir)).unwrap().cache_hit);
    std::fs::rename(dir.join("helper.py"), dir.join("renamed.py")).unwrap();
    let removed = index.resolve(&main, Some(&dir)).unwrap();
    assert!(!removed.cache_hit);
    assert!(removed.paths.is_empty());
    write_file(&dir, "main.py", "import renamed\n");
    let changed = index.resolve(&main, Some(&dir)).unwrap();
    assert!(!changed.cache_hit);
    assert!(changed.paths[0].ends_with("renamed.py"));
    for entry in std::fs::read_dir(&cache).unwrap() {
        std::fs::write(entry.unwrap().path(), "invalid cache").unwrap();
    }
    assert!(!index.resolve(&main, Some(&dir)).unwrap().cache_hit);
    write_file(&dir, "main.py", "import helper\n");
    write_file(&dir, "helper.py", "def answer(): return 7\n");
    assert!(index.resolve(&main, Some(&dir)).unwrap().paths[0].ends_with("helper.py"));
}

#[test]
fn tui_workspace_reuses_the_persisted_dependency_graph_without_source_caching() {
    use clap::Parser;
    let dir = temp_dir("workspace-graph-cache");
    write_file(&dir, "main.py", "import helper\n");
    write_file(&dir, "helper.py", "def answer(): return 42\n");
    let args = cxt_core::cli::Args::parse_from([
        "cxt",
        "main.py",
        "--cache-dir",
        ".cxt/cache",
        "--workspace-root",
        ".",
    ])
    .normalized();
    let first = cxt_core::review::Workspace::load(&args, &dir).unwrap();
    assert!(first.graph_stats.rebuilt >= 2);
    let repeat = cxt_core::review::Workspace::load(&args, &dir).unwrap();
    assert!(repeat.graph_stats.reused >= 2);
    let mut uncached = args.clone();
    uncached.no_cache = true;
    assert_eq!(
        cxt_core::review::Workspace::load(&uncached, &dir)
            .unwrap()
            .graph_stats
            .reused,
        0
    );
}

#[test]
fn unavailable_external_secret_scanner_blocks_delivery() {
    let dir = temp_dir("missing-scanner");
    write_file(&dir, "main.rs", "pub fn answer() {}\n");
    let output = std::process::Command::new(common::cxt_bin())
        .current_dir(&dir)
        .env("PATH", &dir)
        .args([
            "main.rs",
            "--secret-scanner",
            "gitleaks",
            "--output",
            "must-not-exist.md",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(!dir.join("must-not-exist.md").exists());
    assert!(output.stdout.is_empty());
}

#[test]
fn resolver_limits_block_export_instead_of_silently_truncating_dependencies() {
    let dir = temp_dir("resolver-caps");
    write_file(&dir, "go.mod", "module example.local/app\n");
    write_file(
        &dir,
        "main.go",
        "package main\nimport \"example.local/app/helper\"\nfunc main() {}\n",
    );
    for index in 0..1001 {
        write_file(&dir, &format!("helper/file{index}.go"), "package helper\n");
    }
    let out = run_cxt(&dir, &["main.go", "--trace", "--stdout"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("resolver exceeded"));
}

#[test]
fn adaptive_pins_cannot_silently_export_only_a_signature_map() {
    use clap::Parser;
    use std::collections::BTreeSet;
    let dir = temp_dir("adaptive-pin");
    write_file(
        &dir,
        "large.rs",
        &format!(
            "pub fn answer() {{\n{}\n}}",
            "println!(\"large body\");\n".repeat(500)
        ),
    );
    let args = cxt_core::cli::Args::parse_from([
        "cxt",
        "large.rs",
        "--context",
        "adaptive",
        "--budget",
        "500",
    ])
    .normalized();
    let workspace = cxt_core::review::Workspace::load(&args, &dir).unwrap();
    let selected = workspace
        .files
        .iter()
        .map(|f| f.file.path.clone())
        .collect::<BTreeSet<_>>();
    let plan = workspace.plan(&args, &dir, &selected, &selected);
    assert!(
        !plan.checks.is_empty(),
        "omitted pinned implementation must block export"
    );
    if let Ok(result) = plan.result {
        assert!(
            !result
                .packed_files
                .iter()
                .any(|f| f.representation == "map"),
            "pin became a map"
        );
    }
}

#[test]
fn task_metadata_is_redacted_and_fail_policy_checks_it() {
    let dir = temp_dir("task-metadata-secret");
    write_file(&dir, "main.rs", "pub fn answer() {}\n");
    let value = "ExampleOnlyValue".repeat(2);
    let query = format!("answer password={value}");
    let out = run_cxt(
        &dir,
        &["main.rs", "--query", &query, "--format", "json", "--stdout"],
    );
    assert_success(&out);
    assert!(!String::from_utf8_lossy(&out.stdout).contains(&value));
    let failed = run_cxt(
        &dir,
        &[
            "main.rs",
            "--query",
            &query,
            "--secrets",
            "fail",
            "--stdout",
        ],
    );
    assert_eq!(failed.status.code(), Some(4));
    assert!(failed.stdout.is_empty());
}

#[test]
fn symbol_definitions_have_valid_original_ranges_across_supported_languages() {
    let cases = [
        ("rs", "pub fn answer() -> u8 { 42 }", "answer"),
        ("py", "def answer():\n    return 42\n", "answer"),
        ("js", "export function answer() { return 42; }", "answer"),
        (
            "ts",
            "export function answer(): number { return 42; }",
            "answer",
        ),
        (
            "go",
            "package main\nfunc answer() int { return 42 }",
            "answer",
        ),
        ("c", "int answer(void) { return 42; }", "answer"),
        ("cpp", "int answer() { return 42; }", "answer"),
        ("ino", "int answer() { return 42; }", "answer"),
        (
            "java",
            "class App { int answer() { return 42; } }",
            "answer",
        ),
        ("cs", "class App { int Answer() { return 42; } }", "Answer"),
        ("rb", "def answer\n  42\nend\n", "answer"),
        ("php", "<?php function answer() { return 42; }", "answer"),
    ];
    for (extension, source, name) in cases {
        let index =
            cxt_core::symbols::extract(std::path::Path::new(&format!("app.{extension}")), source);
        assert!(index.parsed, "grammar rejected {extension}");
        let symbol = index
            .symbols
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("missing {name} in {extension}: {:?}", index.symbols));
        assert!(source
            .get(symbol.span.start_byte..symbol.span.end_byte)
            .unwrap()
            .contains(name));
        assert!(symbol.span.start_line <= symbol.span.end_line);
    }
}

#[test]
fn exact_budget_is_checked_on_actual_stdout_for_every_output_format() {
    let dir = temp_dir("exact-final-budget");
    for n in 0..6 {
        write_file(
            &dir,
            &format!("source{n}.rs"),
            &format!(
                "pub fn answer_{n}() {{ {} }}",
                "println!(\"unicode λ backticks ```\");".repeat(30)
            ),
        );
    }
    let counter = cxt_core::token::TokenCounter::for_model("cl100k");
    for format in ["markdown", "xml", "json"] {
        let out = run_cxt(
            &dir,
            &[
                ".",
                "--model",
                "cl100k",
                "--max-tokens",
                "1200",
                "--manifest",
                "--format",
                format,
                "--stdout",
            ],
        );
        assert_success(&out);
        let rendered = std::str::from_utf8(&out.stdout).unwrap();
        assert!(
            counter.count(rendered) <= 1200,
            "{format} escaped final budget"
        );
    }
}

#[test]
fn corrupt_cache_and_renamed_files_are_reparsed_safely() {
    let dir = temp_dir("corrupt-index");
    let cache = dir.join("cache");
    let store = cxt_core::index::IndexStore::new(Some(cache.clone()));
    let source = "pub fn answer() -> u8 { 42 }";
    let path = dir.join("original.rs");
    assert!(!store.load(&path, source).cache_hit);
    let entry = std::fs::read_dir(&cache)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(entry, "{not valid cached metadata").unwrap();
    assert!(!store.load(&path, source).cache_hit);
    assert!(store.load(&path, source).cache_hit);
    assert!(!store.load(&dir.join("renamed.rs"), source).cache_hit);
}

#[test]
fn secret_allowlist_requires_an_explicit_matching_exception() {
    let source = format!("{{\"password\":\"{}\"}}", "ExampleOnlyValue".repeat(2));
    let scanner = cxt_core::security::SecretScanner::Builtin;
    assert!(
        !cxt_core::security::configured_findings(&source, scanner, &[])
            .unwrap()
            .is_empty()
    );
    assert!(cxt_core::security::configured_findings(
        &source,
        scanner,
        &["ExampleOnlyValue".into()]
    )
    .unwrap()
    .is_empty());
    assert!(cxt_core::security::configured_findings(&source, scanner, &["[".into()]).is_err());
    assert!(
        cxt_core::security::configured_findings("fn password_check() {}", scanner, &[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn python_parent_relative_import_and_go_package_files_are_traced() {
    let dir = temp_dir("ecosystem-trace");
    write_file(
        &dir,
        "pyproject.toml",
        "[project]\nname = 'example'\nversion = '1.0'\n",
    );
    write_file(
        &dir,
        "package/sub/main.py",
        "from ..helper import answer\nprint(answer())\n",
    );
    write_file(&dir, "package/helper.py", "def answer():\n    return 42\n");
    let py = run_cxt(
        &dir,
        &[
            "package/sub/main.py",
            "--trace",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&py);
    assert!(String::from_utf8_lossy(&py.stdout).contains("helper.py"));
    write_file(&dir, "go.mod", "module example.local/app\n\ngo 1.22\n");
    write_file(
        &dir,
        "main.go",
        "package main\nimport \"example.local/app/helper\"\nfunc main() { helper.Answer() }\n",
    );
    write_file(
        &dir,
        "helper/a.go",
        "package helper\nfunc Answer() int { return Extra() }\n",
    );
    write_file(
        &dir,
        "helper/b.go",
        "package helper\nfunc Extra() int { return 42 }\n",
    );
    let go = run_cxt(
        &dir,
        &["main.go", "--trace", "--format", "json", "--stdout"],
    );
    assert_success(&go);
    let packet: serde_json::Value = serde_json::from_slice(&go.stdout).unwrap();
    assert_eq!(packet["files"].as_array().unwrap().len(), 3);
}

#[test]
fn cargo_workspace_path_dependencies_resolve_without_confusing_registry_crates() {
    let dir = temp_dir("cargo-workspace");
    write_file(&dir, "Cargo.toml", "[workspace]\nmembers = ['app', 'helper']\n[workspace.dependencies]\nhelper = { path = 'helper' }\n");
    write_file(&dir, "app/Cargo.toml", "[package]\nname = 'app'\nversion = '1.0.0'\n[dependencies]\nhelper = { workspace = true }\n");
    write_file(
        &dir,
        "app/src/main.rs",
        "use helper::answer;\nfn main() { answer(); }\n",
    );
    write_file(
        &dir,
        "helper/Cargo.toml",
        "[package]\nname = 'helper'\nversion = '1.0.0'\n",
    );
    write_file(&dir, "helper/src/lib.rs", "pub fn answer() -> u8 { 42 }\n");
    let out = run_cxt(
        &dir,
        &[
            "app/src/main.rs",
            "--trace",
            "--workspace-root",
            ".",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("fn answer"));
}

#[test]
fn go_workspace_modules_and_java_standard_source_roots_resolve() {
    let dir = temp_dir("go-java-workspaces");
    write_file(&dir, "go.work", "go 1.22\nuse (\n ./app\n ./shared\n)\n");
    write_file(&dir, "app/go.mod", "module example.local/app\n");
    write_file(&dir, "shared/go.mod", "module example.local/shared\n");
    write_file(
        &dir,
        "app/main.go",
        "package main\nimport \"example.local/shared\"\nfunc main() { shared.Answer() }\n",
    );
    write_file(
        &dir,
        "shared/answer.go",
        "package shared\nfunc Answer() int { return 42 }\n",
    );
    let out = run_cxt(
        &dir,
        &[
            "app/main.go",
            "--trace",
            "--workspace-root",
            ".",
            "--stdout",
        ],
    );
    assert_success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("func Answer"));
    write_file(&dir, "pom.xml", "<project/>\n");
    write_file(
        &dir,
        "src/main/java/app/Main.java",
        "package app; import helper.Answer; class Main {}\n",
    );
    write_file(
        &dir,
        "src/main/java/helper/Answer.java",
        "package helper; class Answer {}\n",
    );
    let out = run_cxt(
        &dir,
        &["src/main/java/app/Main.java", "--trace", "--stdout"],
    );
    assert_success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("class Answer"));
}

#[test]
fn workspace_scope_ignores_configuration_extensions_outside_its_root() {
    let dir = temp_dir("scoped-config");
    write_file(
        &dir,
        "outside.json",
        "{\"compilerOptions\":{\"baseUrl\":\"./workspace\",\"paths\":{\"@private/*\":[\"*\"]}}}",
    );
    write_file(
        &dir,
        "workspace/tsconfig.json",
        "{\"extends\":\"../outside.json\"}",
    );
    write_file(
        &dir,
        "workspace/main.ts",
        "import { answer } from '@private/helper'; console.log(answer);\n",
    );
    write_file(&dir, "workspace/helper.ts", "export const answer = 42;\n");
    let out = run_cxt(
        &dir,
        &[
            "workspace/main.ts",
            "--trace",
            "--workspace-root",
            "workspace",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["files"].as_array().unwrap().len(), 1);
}

#[test]
fn focused_budget_keeps_small_target_without_its_large_keyword_distractor() {
    let dir = temp_dir("focused-budget");
    write_file(&dir, "audio.rs", &format!("pub fn route_volume(client: u8) -> u8 {{ client }}\n\npub fn debug_volume() {{\n{}\n}}", "println!(\"volume debug log\");\n".repeat(500)));
    let out = run_cxt(
        &dir,
        &[
            "audio.rs",
            "--query",
            "route volume client",
            "--context",
            "focused",
            "--max-tokens",
            "1000",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(doc["files"][0]["content"]
        .as_str()
        .unwrap()
        .contains("fn route_volume"));
    assert!(!doc["files"][0]["content"]
        .as_str()
        .unwrap()
        .contains("fn debug_volume"));
}

#[test]
fn evaluation_reports_required_source_coverage_without_claiming_task_success() {
    let dir = temp_dir("evaluation");
    write_file(
        &dir,
        "repo/audio.rs",
        "    pub fn set_volume() -> u8 { 42 }\n",
    );
    let corpus = serde_json::json!({"schema":"cxt.eval.v1","dataset_kind":"synthetic","budgets":[1000],"tasks":[{"id":"volume","root":"repo","query":"set volume","required_files":["audio.rs"],"required_symbols":[{"path":"audio.rs","name":"Audio::set_volume","start_line":1,"end_line":1}]}]});
    write_file(&dir, "corpus.json", &corpus.to_string());
    let out = run_cxt(&dir, &["--evaluate", "corpus.json"]);
    assert_success(&out);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(report["runs"].as_array().unwrap().len() >= 3);
    assert_eq!(report["downstream_task_success"], "not_measured");
    assert!(report["runs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["file_recall"] == 1.0));
    assert!(report["runs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["symbol_recall"] == 1.0));
    let mut stale = corpus;
    stale["tasks"][0]["required_symbols"][0]["source_sha256"] = serde_json::json!("0".repeat(64));
    write_file(&dir, "stale.json", &stale.to_string());
    let rejected = run_cxt(&dir, &["--evaluate", "stale.json"]);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("source revision"));
}

#[test]
#[ignore = "requires the separately installed Gitleaks executable"]
fn installed_gitleaks_detects_a_provider_not_covered_by_builtin_patterns() {
    let secret = format!("glpat-{}", "aB3dE6gH9jK2mN5pQ8rS");
    let source = format!("credential = \"{secret}\"\n");
    assert!(cxt_core::security::find_secrets(&source).is_empty());
    let findings = cxt_core::security::configured_findings(
        &source,
        cxt_core::security::SecretScanner::Gitleaks,
        &[],
    )
    .unwrap();
    assert!(!findings.is_empty());
    let (redacted, _) = cxt_core::security::redact_findings(&source, &findings);
    assert!(!redacted.contains(&secret));
}

#[test]
fn json_yaml_credentials_are_redacted_and_explicit_examples_can_be_allowlisted() {
    let dir = temp_dir("structured-secrets");
    let secret = "a".repeat(24);
    write_file(&dir, "settings.yaml", &format!("api_key: '{secret}'\n"));
    write_file(
        &dir,
        "settings.json",
        &format!("{{\"client_secret\": \"{secret}\"}}"),
    );
    let out = run_cxt(
        &dir,
        &[
            "settings.yaml",
            "settings.json",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    assert!(!String::from_utf8_lossy(&out.stdout).contains(&secret));
    let allowed = run_cxt(
        &dir,
        &[
            "settings.yaml",
            "--secret-allow",
            "^api_key: 'a{24}'$",
            "--secrets",
            "fail",
            "--stdout",
        ],
    );
    assert_success(&allowed);
}

#[test]
fn local_workspace_package_exports_resolve_without_node_modules() {
    let dir = temp_dir("node-workspace");
    write_file(&dir, "package.json", "{\"workspaces\":[\"packages/*\"]}");
    write_file(
        &dir,
        "packages/audio/package.json",
        "{\"name\":\"@demo/audio\",\"exports\":{\".\":{\"types\":\"./src/index.ts\"}}}",
    );
    write_file(
        &dir,
        "packages/audio/src/index.ts",
        "export const volume = 42;\n",
    );
    write_file(
        &dir,
        "main.ts",
        "import { volume } from '@demo/audio'; console.log(volume);\n",
    );
    let out = run_cxt(
        &dir,
        &["main.ts", "--trace", "--format", "json", "--stdout"],
    );
    assert_success(&out);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["files"].as_array().unwrap().len(), 2);
}

#[test]
fn tracing_resolves_jsonc_inheritance_and_alias_fallback_targets() {
    let dir = temp_dir("jsonc-alias");
    write_file(&dir, "tsconfig.base.json", "{ /* aliases */ \"compilerOptions\": { \"baseUrl\": \".\", \"paths\": { \"@app/*\": [\"missing/*\", \"src/*\"], }, }, }");
    write_file(
        &dir,
        "tsconfig.json",
        "{ \"extends\": \"./tsconfig.base.json\", }",
    );
    write_file(
        &dir,
        "src/main.ts",
        "import { answer } from '@app/helper'; console.log(answer);\n",
    );
    write_file(&dir, "src/helper.ts", "export const answer = 42;\n");
    let out = run_cxt(
        &dir,
        &["src/main.ts", "--trace", "--format", "json", "--stdout"],
    );
    assert_success(&out);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["files"].as_array().unwrap().len(), 2);
}

#[test]
fn adaptive_budget_retains_an_orientation_map_when_implementation_is_too_large() {
    let dir = temp_dir("adaptive-budget");
    write_file(
        &dir,
        "large.rs",
        &format!(
            "pub fn route_volume() {{\n{}\n}}",
            "println!(\"large implementation\");\n".repeat(500)
        ),
    );
    let out = run_cxt(
        &dir,
        &[
            "large.rs",
            "--context",
            "adaptive",
            "--max-tokens",
            "300",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["files"][0]["representation"], "map");
    assert!(!doc["files"][0]["content"]
        .as_str()
        .unwrap()
        .contains("large implementation"));
}

#[test]
fn symbol_signals_outweigh_repeated_logging_terms() {
    let dir = temp_dir("hybrid-ranking");
    write_file(&dir, "logic.ts", "export function routeClientVolume(clientId: string, level: number) { return { clientId, level }; }\n");
    write_file(&dir, "log.txt", &"client volume routing\n".repeat(200));
    let out = run_cxt(
        &dir,
        &[
            ".",
            "--query",
            "client volume routing",
            "--max-files",
            "1",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["files"][0]["path"], "logic.ts");
}

#[test]
fn persistent_index_reuses_parsing_but_invalidates_changed_source() {
    let dir = temp_dir("index-cache");
    let source = "pub fn answer() { println!(\"PRIVATE_BODY_MARKER\"); }";
    let store = cxt_core::index::IndexStore::new(Some(dir.clone()));
    let path = dir.join("answer.rs");
    let first = store.load(&path, source);
    assert!(!first.cache_hit);
    let warm = store.load(&path, source);
    assert!(warm.cache_hit);
    assert_eq!(warm.index.symbols[0].name, "answer");
    let changed = store.load(&path, "pub fn changed() {}");
    assert!(!changed.cache_hit);
    assert_eq!(changed.index.symbols[0].name, "changed");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!text.contains("PRIVATE_BODY_MARKER"));
    }
}

#[test]
fn named_exact_tokenizer_counts_literal_special_token_text() {
    let counter = cxt_core::token::TokenCounter::for_model("cl100k");
    assert!(counter.is_exact());
    assert_eq!(counter.count("hello world"), 2);
    assert_eq!(counter.count("<|endoftext|>"), 7);
}

#[test]
fn focused_source_retains_export_and_decorator_wrappers() {
    let dir = temp_dir("focused-wrappers");
    for (path, source, name) in [
        (
            "api.ts",
            "export default async function routeVolume(client: number) { return client + 1; }",
            "routeVolume",
        ),
        (
            "api.py",
            "@instrument\ndef route_volume(client):\n    return client + 1",
            "route_volume",
        ),
        (
            "constants.ts",
            "export const volume = (client: number) => client + 1;",
            "volume",
        ),
    ] {
        write_file(&dir, path, source);
        let out = run_cxt(
            &dir,
            &[
                path,
                "--symbol",
                name,
                "--context",
                "focused",
                "--format",
                "json",
                "--stdout",
            ],
        );
        assert_success(&out);
        let packet: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(packet["files"][0]["content"], source, "{path}");
    }
}

#[test]
fn focused_context_exports_named_implementation_and_original_locations() {
    let dir = temp_dir("focused");
    write_file(&dir, "audio.rs", "use std::collections::HashMap;\n\npub fn set_volume(client_id: &str, value: u8) { println!(\"{} {}\", client_id, value); }\n\npub fn unrelated() { println!(\"distractor\"); }\n");
    let out = run_cxt(
        &dir,
        &[
            "audio.rs",
            "--context",
            "focused",
            "--symbol",
            "set_volume",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let file = &doc["files"][0];
    assert!(file["content"].as_str().unwrap().contains("fn set_volume"));
    assert!(file["content"]
        .as_str()
        .unwrap()
        .contains("use std::collections"));
    assert!(!file["content"].as_str().unwrap().contains("fn unrelated"));
    assert!(file["spans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["start_line"] == 3));
}
