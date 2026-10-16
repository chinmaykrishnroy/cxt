#[allow(dead_code)]
mod common;

use common::{assert_success, run_cxt, write_file};

#[test]
fn evaluation_distinguishes_secret_refusals_without_recording_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let secret = "Q7x".repeat(14);
    write_file(
        dir.path(),
        "repo/audio.py",
        &format!(
            "def change_volume():\n    return 42\n\ndef unused():\n    api_key = '{secret}'\n"
        ),
    );
    let corpus = serde_json::json!({"schema":"cxt.eval.v1","dataset_kind":"synthetic refusal regression","budgets":[4000],"contexts":["focused"],"tasks":[{"id":"secret","root":"repo","query":"volume","required_files":["audio.py"],"required_symbols":[{"path":"audio.py","name":"change_volume","start_line":1,"end_line":2}]}]});
    write_file(dir.path(), "corpus.json", &corpus.to_string());
    let out = run_cxt(dir.path(), &["--evaluate", "corpus.json"]);
    assert_success(&out);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    for run in report["runs"].as_array().unwrap() {
        assert_eq!(
            run["success"], false,
            "original-source fail policy is unchanged"
        );
        assert_eq!(run["failure_kind"], "secret_policy");
        assert_eq!(run["exit_code"], 4);
        assert!(run["elapsed_ms"].as_f64().unwrap() > 0.0);
    }
    assert!(!String::from_utf8_lossy(&out.stdout).contains(&secret));
    assert!(!String::from_utf8_lossy(&out.stderr).contains(&secret));
}

#[test]
fn evaluation_distinguishes_empty_queries_from_packet_budget_failures() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "repo/audio.py",
        "def change_volume():\n    return 42\n",
    );
    for (query, budget, context, expected) in [
        ("nonexistentzzzz", 4000, "focused", "no_query_match"),
        ("volume", 1, "focused", "token_budget"),
        ("volume", 100, "complete", "token_budget"),
    ] {
        let corpus = serde_json::json!({"schema":"cxt.eval.v1","dataset_kind":"synthetic failure categories","budgets":[budget],"contexts":[context],"tasks":[{"id":"failure","root":"repo","query":query,"required_files":["audio.py"],"required_symbols":[]}]});
        write_file(dir.path(), "corpus.json", &corpus.to_string());
        let out = run_cxt(dir.path(), &["--evaluate", "corpus.json"]);
        assert_success(&out);
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        for run in report["runs"].as_array().unwrap() {
            assert_eq!(run["failure_kind"], expected);
            assert_eq!(run["exit_code"], 2);
        }
    }
}
