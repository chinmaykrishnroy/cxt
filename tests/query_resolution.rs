#[allow(dead_code)]
mod common;

use common::{assert_success, run_cxt, write_file};
use serde_json::Value;

#[test]
fn python_class_body_imports_are_not_lexical_bindings_inside_methods() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "class Screen:\n    from state import commit as update\n\n    def change_volume(self, value):\n        return update(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit(value):\n    return value + 1\n",
    );
    write_file(
        dir.path(),
        "events.py",
        "from state import commit as post\n\ndef dispatch(value):\n    return post(value)\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        !doc["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "events.py"),
        "Python methods do not implicitly capture their class namespace"
    );
}

#[test]
fn typescript_namespace_imports_do_not_guess_an_unknown_object_receiver() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.ts", "import * as st from './state';\nexport function changeVolume(value: number) { return st.commit(value); }\n");
    write_file(
        dir.path(),
        "state.ts",
        "export function commit(value: number) { return value + 1; }\n",
    );
    write_file(dir.path(), "events.ts", "import * as storage from './state';\nexport function dispatch(value: number) { return storage.commit(value); }\n");
    write_file(
        dir.path(),
        "unrelated.ts",
        "export function invoke(other: any, value: number) { return other.commit(value); }\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(files.iter().any(|f| f["path"] == "events.ts"));
    assert!(!files.iter().any(|f| f["path"] == "unrelated.ts"));
}

#[test]
fn changing_an_alias_configuration_rebinds_unchanged_callers_with_cache_enabled() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.ts", "import { commit as persist } from '@state';\nexport function changeVolume(value: number) { return persist(value); }\n");
    for target in ["first", "second"] {
        write_file(
            dir.path(),
            &format!("{target}/state.ts"),
            "export function commit(value: number) { return value + 1; }\n",
        );
        write_file(dir.path(), &format!("{target}/events.ts"), "import { commit as update } from './state';\nexport function dispatch(value: number) { return update(value); }\n");
    }
    for (wanted, other) in [("first", "second"), ("second", "first")] {
        write_file(
            dir.path(),
            "tsconfig.json",
            &format!(
                r#"{{"compilerOptions":{{"baseUrl":".","paths":{{"@state":["{wanted}/state"]}}}}}}"#
            ),
        );
        let out = run_cxt(
            dir.path(),
            &[
                "--query",
                "volume",
                "--context",
                "focused",
                "--format",
                "json",
                "--stdout",
                "--cache-dir",
                "target/cache",
            ],
        );
        assert_success(&out);
        let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
        let paths: Vec<_> = doc["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["path"].as_str().unwrap().replace('\\', "/"))
            .collect();
        assert!(paths.contains(&format!("{wanted}/events.ts")), "{paths:?}");
        assert!(!paths.contains(&format!("{other}/events.ts")), "{paths:?}");
    }
}

#[test]
fn typescript_default_imports_follow_the_default_export_not_the_alias_name() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.ts", "import persist from './state';\nexport function changeVolume(value: number) { return persist(value); }\n");
    write_file(
        dir.path(),
        "state.ts",
        "export default function commit(value: number) { return value + 1; }\n",
    );
    write_file(dir.path(), "events.ts", "import update from './state';\nexport function dispatch(value: number) { return update(value); }\n");
    write_file(
        dir.path(),
        "unrelated.ts",
        "export function update(value: number) { throw new Error('unrelated'); }\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(files.iter().any(|f| f["path"] == "events.ts"));
    assert!(!files.iter().any(|f| f["path"] == "unrelated.ts"));
}

#[test]
fn an_unbound_python_name_does_not_resolve_to_an_unrelated_class_method() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "audio.py",
        "def change_volume(value):\n    return commit(value)\n",
    );
    write_file(
        dir.path(),
        "storage.py",
        "class Storage:\n    def commit(self, value):\n        return value + 1\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(!doc["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == "storage.py"));
}

#[test]
fn unresolved_imports_are_explained_instead_of_guessing_a_global_target() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "audio.py",
        "from absent_module import commit\n\ndef change_volume(value):\n    return commit(value)\n",
    );
    write_file(
        dir.path(),
        "unrelated.py",
        "def commit(value):\n    return value + 1\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--manifest",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(!doc["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == "unrelated.py"));
    let reasons = doc["files"][0]["reasons"].to_string();
    assert!(
        reasons.contains("query-unresolved")
            && reasons.contains("audio.py:4")
            && reasons.contains("commit"),
        "{reasons}"
    );
}

#[test]
fn focused_symbol_does_not_add_a_definition_shadowed_by_a_parameter() {
    let source = "def commit(value):\n    return value + 1\n\ndef change_volume(commit, value):\n    return commit(value)\n";
    let index = cxt_core::symbols::extract(std::path::Path::new("audio.py"), source);
    let (rendered, _, _) = cxt_core::symbols::represent(
        &index,
        source,
        cxt_core::symbols::ContextLevel::Focused,
        None,
        Some("change_volume"),
    )
    .unwrap();
    assert!(rendered.contains("def change_volume"));
    assert!(
        !rendered.contains("def commit"),
        "focused helper retention must respect the same binding rules as query traversal"
    );
}

#[test]
fn typescript_block_bindings_shadow_imports_only_inside_their_scope() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.ts", "import { commit as persist } from './state';\nexport function changeVolume(value: number) { return persist(value); }\n");
    write_file(
        dir.path(),
        "state.ts",
        "export function commit(value: number) { return value + 1; }\n",
    );
    write_file(dir.path(), "events.ts", "import { commit as update } from './state';\nexport function dispatch(callback: any, value: number) { { const update = callback; return update(value); } }\n");
    write_file(dir.path(), "outside.ts", "import { commit as update } from './state';\nexport function refresh(callback: any, value: number) { { const update = callback; update(value); } return update(value); }\n");
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(files.iter().any(|f| f["path"] == "outside.ts"));
    assert!(
        !files.iter().any(|f| f["path"] == "events.ts"),
        "a block-local callback is not the imported function"
    );
}

#[test]
fn a_function_local_assignment_shadows_an_import_for_the_entire_python_scope() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "from state import commit as persist\n\ndef change_volume(value):\n    return persist(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit(value):\n    return value + 1\n",
    );
    write_file(dir.path(), "events.py", "from state import commit as update\n\ndef dispatch(callback, value):\n    update = callback\n    return update(value)\n");
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        !doc["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "events.py"),
        "rebinding is not evidence of calling the imported state helper"
    );
}

#[test]
fn relationship_reasons_identify_call_sites_and_binding_evidence() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "from state import commit as persist\n\ndef change_volume(value):\n    return persist(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit(value):\n    return value + 1\n",
    );
    write_file(
        dir.path(),
        "events.py",
        "from state import commit as update\n\ndef dispatch(value):\n    return update(value)\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--manifest",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let event = doc["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == "events.py")
        .unwrap();
    let reasons = event["reasons"].to_string();
    assert!(
        reasons.contains("CALLS")
            && reasons.contains("CALLED_BY")
            && reasons.contains("import-bound"),
        "{reasons}"
    );
    assert!(
        reasons.contains("audio.py:4") && reasons.contains("events.py:4"),
        "{reasons}"
    );
    assert!(
        !reasons.contains("compiler-resolved"),
        "syntax binding is not compiler proof"
    );
}

#[test]
fn module_alias_members_do_not_bind_unrelated_object_methods() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "audio.py",
        "import state as st\n\ndef change_volume(value):\n    return st.commit(value)\n",
    );
    write_file(
        dir.path(),
        "state.py",
        "def commit(value):\n    return value + 1\n",
    );
    write_file(
        dir.path(),
        "events.py",
        "import state as storage\n\ndef dispatch(value):\n    return storage.commit(value)\n",
    );
    write_file(dir.path(), "noise.py", "def commit(value):\n    raise RuntimeError('unrelated')\n\ndef invoke(other, value):\n    return other.commit(value)\n");
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(files.iter().any(|file| file["path"] == "events.py"
        && file["content"]
            .as_str()
            .unwrap()
            .contains("def dispatch(value):\n    return storage.commit(value)")));
    assert!(!files.iter().any(|file| file["path"] == "noise.py"));
}

#[test]
fn typescript_named_import_aliases_link_the_exported_definition() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.ts", "import { commit as persist } from './state';\nexport function changeVolume(value: number) { return persist(value); }\n");
    write_file(
        dir.path(),
        "state.ts",
        "export function commit(value: number) { return value + 1; }\n",
    );
    write_file(dir.path(), "events.ts", "import { commit as update } from './state';\nexport function dispatch(value: number) { return update(value); }\n");
    write_file(
        dir.path(),
        "unrelated.ts",
        "export function commit(value: number) { throw new Error('unrelated'); }\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(files.iter().any(|file| file["path"] == "events.ts"
        && file["content"]
            .as_str()
            .unwrap()
            .contains("export function dispatch(value: number) { return update(value); }")));
    assert!(!files.iter().any(|file| file["path"] == "unrelated.ts"));
}

#[test]
fn a_parameter_shadowing_an_import_is_not_a_shared_caller() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "from state import commit_level as persist\n\ndef change_volume(value):\n    return persist(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit_level(value):\n    return value + 1\n",
    );
    write_file(dir.path(), "events.py", "from state import commit_level as update\n\ndef dispatch(update, value):\n    return update(value)\n");
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        !doc["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "events.py"),
        "a caller-supplied callback is not the imported helper"
    );
}

#[test]
fn imported_aliases_link_shared_callers_without_global_name_guessing() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "audio.py", "from state import commit_level as persist\n\ndef change_volume(value):\n    return persist(value)\n");
    write_file(
        dir.path(),
        "state.py",
        "def commit_level(value):\n    return value + 1\n",
    );
    write_file(dir.path(), "events.py", "from state import commit_level as update\n\ndef dispatch(value):\n    return update(value)\n");
    write_file(
        dir.path(),
        "unrelated.py",
        "def commit_level(value):\n    raise RuntimeError('unrelated subsystem')\n",
    );
    let out = run_cxt(
        dir.path(),
        &[
            "--query",
            "volume",
            "--context",
            "focused",
            "--format",
            "json",
            "--stdout",
            "--no-cache",
        ],
    );
    assert_success(&out);
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files = doc["files"].as_array().unwrap();
    assert!(
        files.iter().any(|file| file["path"] == "events.py"
            && file["content"]
                .as_str()
                .unwrap()
                .contains("def dispatch(value):\n    return update(value)")),
        "import aliases should lead back to the real shared-state caller"
    );
    assert!(!files.iter().any(|file| file["path"] == "unrelated.py"));
}
