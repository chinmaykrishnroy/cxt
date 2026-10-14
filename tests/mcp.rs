#[allow(dead_code)]
mod common;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn read_only_mcp_rejects_policy_overrides_and_invalid_tool_arguments() {
    let dir = common::temp_dir("mcp-policy");
    common::write_file(&dir, "main.rs", "pub fn answer() {}\n");
    common::write_file(
        &dir,
        ".cxt.toml",
        "global_ignore = ['*.rs']\nsecret_policy = 'allow'\n",
    );
    let mut server = cxt_core::mcp::Server::new(&dir).unwrap();
    server.request(serde_json::json!({"jsonrpc":"2.0","id":0,"method":"initialize"}));
    server.request(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    for arguments in [
        serde_json::json!({"secrets":"allow"}),
        serde_json::json!({"max_tokens":0}),
        serde_json::json!({"query":42}),
        serde_json::json!({"paths":["../main.rs"]}),
        serde_json::json!({"output":"stolen.json"}),
    ] {
        let reply = server.request(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_context","arguments":arguments}})).unwrap();
        assert_eq!(reply["result"]["isError"], true);
    }
    assert!(!dir.join("stolen.json").exists());
    let reply = server.request(serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_symbol","arguments":{"paths":["main.rs"],"symbol":"answer"}}})).unwrap();
    assert_eq!(reply["result"]["isError"], false);
    assert_eq!(
        reply["result"]["structuredContent"]["files"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn stdio_mcp_serves_structured_context_and_rejects_workspace_escape() {
    let dir = common::temp_dir("mcp");
    common::write_file(&dir, "main.rs", "pub fn answer() -> u8 { 42 }\n");
    let mut child = Command::new(common::cxt_bin())
        .current_dir(&dir)
        .args(["--mcp", "."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let requests = [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_context","arguments":{"paths":["../outside"]}}}),
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_symbol","arguments":{"paths":["main.rs"],"symbol":"answer"}}}),
    ];
    let input = requests
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    common::assert_success(&output);
    let replies: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(replies.len(), 4);
    assert_eq!(replies[2]["result"]["isError"], true);
    assert!(
        replies[3]["result"]["structuredContent"]["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("fn answer")
    );
}
