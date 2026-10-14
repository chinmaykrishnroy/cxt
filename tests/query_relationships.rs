#[allow(dead_code)]
mod common;

use common::{assert_success, run_cxt, write_file};
use serde_json::Value;

#[test]
fn query_prioritizes_an_explicit_module_over_generic_api_chatter() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "tarfs/fs.py",
        "def independent_handles():\n    return 'separate positions'\n",
    );
    for i in 0..3 {
        write_file(
            dir.path(),
            &format!("backend_{i}.py"),
            &format!(
                "def open_read_close_handles_{i}():\n{}    return 42\n",
                "    print('open read close handles')\n".repeat(90)
            ),
        );
    }
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "tarfs independent handles open read close",
            "--context",
            "focused",
            "--model",
            "cl100k",
            "--budget",
            "2500",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        doc["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |file| file["path"].as_str().unwrap().replace('\\', "/") == "tarfs/fs.py"
                    && file["content"]
                        .as_str()
                        .unwrap()
                        .contains("def independent_handles()")
            ),
        "explicit module should survive generic API anchors"
    );
}

#[test]
fn focused_query_leaves_room_for_multiple_primary_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        write_file(dir.path(), &format!("palette_{i}.py"), &format!("def apply_palette_color_{i}(value):\n{}    return value\n\ndef palette_debug_{i}():\n{}\ndef palette_log_{i}():\n{}", "    print('palette color')\n".repeat(35), "    print('palette diagnostic')\n".repeat(100), "    print('palette logging')\n".repeat(150)));
    }
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "apply palette color",
            "--context",
            "focused",
            "--model",
            "cl100k",
            "--budget",
            "6000",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    for i in 0..3 {
        assert!(
            files
                .iter()
                .any(|file| file["path"] == format!("palette_{i}.py")
                    && file["content"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("def apply_palette_color_{i}(value)"))),
            "all three primary implementations should share the context budget; exported paths: {:?}", files.iter().map(|file| &file["path"]).collect::<Vec<_>>()
        );
    }
}

#[test]
fn named_rust_field_retains_its_attributes_without_exporting_the_whole_struct() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "options.rs", "pub struct Options {\n    #[arg(long, allow_hyphen_values = true)]\n    pub format: Option<String>,\n    pub unrelated: bool,\n}\n");
    let out = run_cxt(
        dir.path(),
        &[
            "options.rs",
            "--symbol",
            "Options::format",
            "--context",
            "focused",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let file = &doc["files"][0];
    let content = file["content"].as_str().unwrap();
    assert!(content
        .contains("#[arg(long, allow_hyphen_values = true)]\n    pub format: Option<String>,"));
    assert!(!content.contains("unrelated"));
    assert!(file["spans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["start_line"] == 2 && s["end_line"] == 3));
}

#[test]
fn query_keeps_primary_implementation_before_cheap_supporting_files() {
    let dir = tempfile::tempdir().unwrap();
    let implementation = format!(
        "def apply_palette_color(index):\n{}    return index\n",
        "    print('preserve palette color')\n".repeat(100)
    );
    write_file(dir.path(), "src/painting.py", &implementation);
    for i in 0..25 {
        write_file(
            dir.path(),
            &format!("notes/{i}.txt"),
            "The palette color index must preserve explicit black color.\n",
        );
    }
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "apply palette color index preserve black",
            "--context",
            "focused",
            "--model",
            "cl100k",
            "--budget",
            "2000",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        doc["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |file| file["path"].as_str().unwrap().replace('\\', "/") == "src/painting.py"
                    && file["content"]
                        .as_str()
                        .unwrap()
                        .contains(implementation.trim_end())
            ),
        "cheap supporting files must not crowd out the highest-scoring implementation"
    );
}

#[test]
fn query_follows_shared_state_back_to_a_nonmatching_handler() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "from state import commit_level\n\ndef change_volume(value):\n    return commit_level(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit_level(value):\n    return value + 1\n",
    );
    let handler = "def dispatch(value):\n    return commit_level(value)\n";
    write_file(
        dir.path(),
        "events.py",
        &format!(
            "from state import commit_level\n\n{handler}\ndef unrelated():\n{}",
            "    print('diagnostic noise')\n".repeat(1500)
        ),
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--model",
            "cl100k",
            "--budget",
            "4000",
            "--manifest",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    let file = files
        .iter()
        .find(|f| f["path"] == "events.py")
        .expect("shared-state callers without query words must not disappear");
    let content = file["content"].as_str().unwrap();
    assert!(
        content.contains(handler.trim_end()),
        "export the actual handler implementation"
    );
    assert!(
        !content.contains("def unrelated"),
        "do not replace relationship evidence with a whole-file fallback"
    );
    assert_eq!(file["representation"], "focused");
    assert!(file["spans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["start_line"] == 3 && s["end_line"] == 4));
}

#[test]
fn ambiguous_reference_names_do_not_pull_unrelated_implementations() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "from state import commit_level\n\ndef change_volume(value):\n    return commit_level(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit_level(value):\n    return value + 1\n",
    );
    write_file(
        dir.path(),
        "unrelated.py",
        "def commit_level(value):\n    raise RuntimeError('unrelated service')\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(
        files.iter().any(|f| f["path"] == "state.py"),
        "supported static imports still expand"
    );
    assert!(
        !files.iter().any(|f| f["path"] == "unrelated.py"),
        "a same-name definition is not a resolved reference"
    );
}

#[test]
fn strings_comments_and_word_fragments_are_not_definition_references() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "def change_volume():\n    # reset_everything is not called\n    print('initialize')\n    send = 1\n    request = 2\n    return send + request\n\ndef initialize():\n    return 'unrelated string match'\n\ndef reset_everything():\n    return 'unrelated comment match'\n\ndef send_request():\n    return 'unrelated split-word match'\n");
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let content = doc["files"][0]["content"].as_str().unwrap();
    assert!(content.contains("def change_volume"));
    for unrelated in ["initialize", "reset_everything", "send_request"] {
        assert!(
            !content.contains(&format!("def {unrelated}")),
            "included non-reference {unrelated}"
        );
    }
}

#[test]
fn disabling_expansion_keeps_the_query_local() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "from state import commit_level\n\ndef change_volume(value):\n    return commit_level(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit_level(value):\n    return value + 1\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--no-query-expansion",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["files"].as_array().unwrap().len(), 1);
    assert_eq!(doc["files"][0]["path"], "audio.py");
}

#[test]
fn references_in_another_language_do_not_make_a_definition_ambiguous() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "audio.py",
        include_str!("../eval/fixtures/relationships-python/audio.py"),
    );
    write_file(
        dir.path(),
        "state.py",
        include_str!("../eval/fixtures/relationships-python/state.py"),
    );
    write_file(
        dir.path(),
        "events.py",
        include_str!("../eval/fixtures/relationships-python/events.py"),
    );
    write_file(
        dir.path(),
        "unrelated.rs",
        "fn commit_level(value: u8) -> u8 { value }\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(
        files.iter().any(|f| f["path"] == "events.py"),
        "unrelated languages must not hide a shared-state caller"
    );
    assert!(!files.iter().any(|f| f["path"] == "unrelated.rs"));
}

#[test]
fn query_relationships_work_across_typescript_and_tsx() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "audio.ts",
        include_str!("../eval/fixtures/relationships-typescript/audio.ts"),
    );
    write_file(
        dir.path(),
        "state.ts",
        include_str!("../eval/fixtures/relationships-typescript/state.ts"),
    );
    write_file(
        dir.path(),
        "events.tsx",
        include_str!("../eval/fixtures/relationships-typescript/events.ts"),
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let event = doc["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "events.tsx")
        .unwrap();
    assert!(event["content"]
        .as_str()
        .unwrap()
        .contains("export function dispatch(value: number) {\n    return commitLevel(value);\n}"));
    assert!(!event["content"]
        .as_str()
        .unwrap()
        .contains("function unrelated"));
}

#[test]
fn high_fanout_helpers_do_not_expand_into_every_caller() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "audio.py",
        include_str!("../eval/fixtures/relationships-python/audio.py"),
    );
    write_file(
        dir.path(),
        "state.py",
        include_str!("../eval/fixtures/relationships-python/state.py"),
    );
    for i in 0..40 {
        write_file(dir.path(), &format!("handler_{i}.py"), &format!("from state import commit_level\n\ndef dispatch_{i}(value):\n    return commit_level(value)\n"));
    }
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--max-files",
            "0",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["files"].as_array().unwrap().len(), 2);
}

#[test]
fn relationship_hints_do_not_hide_other_lexically_relevant_implementations() {
    let dir = tempfile::tempdir().unwrap();
    let source = (0..4)
        .map(|i| format!("def volume_{i}():\n    return {i}\n\n"))
        .collect::<String>();
    write_file(dir.path(), "audio.py", &source);
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let content = doc["files"][0]["content"].as_str().unwrap();
    for i in 0..4 {
        assert!(
            content.contains(&format!("def volume_{i}():\n    return {i}")),
            "relationship seed limits must not remove ordinary query matches"
        );
    }
}

#[test]
fn budget_omissions_do_not_weaken_candidate_wide_fail_policy() {
    let dir = tempfile::tempdir().unwrap();
    let secret = "R8k".repeat(16);
    write_file(
        dir.path(),
        "audio.py",
        "def change_volume():\n    return 42\n",
    );
    write_file(
        dir.path(),
        "volume_notes.txt",
        &format!(
            "api_key = '{secret}'\n{}",
            "unrelated diagnostic noise\n".repeat(4000)
        ),
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--secrets",
            "fail",
            "--model",
            "cl100k",
            "--budget",
            "2000",
            "--manifest",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stdout).contains(&secret));
    assert!(!String::from_utf8_lossy(&out.stderr).contains(&secret));
}

#[test]
fn failed_span_preparation_cannot_discard_original_secret_findings() {
    let dir = tempfile::tempdir().unwrap();
    let secret = "R8k".repeat(16);
    write_file(
        dir.path(),
        "audio.py",
        "def change_volume():\n    return 42\n",
    );
    write_file(
        dir.path(),
        "diagnostics.py",
        &format!(
            "def volume_diagnostics():\n    api_key = '{secret}'\n{}",
            "    print('diagnostic noise')\n".repeat(4000)
        ),
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--secrets",
            "fail",
            "--model",
            "cl100k",
            "--budget",
            "2000",
            "--no-cache",
            "--format",
            "json",
            "--stdout",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(4),
        "candidate findings must survive failed representation: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stderr).contains(&secret));
}
