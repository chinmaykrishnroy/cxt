#[allow(dead_code)]
mod common;
use std::io::Write;
use std::process::{Command, Stdio};

fn call_tool(
    root: &std::path::Path,
    name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    let mut server = cxt_core::mcp::Server::new(root).unwrap();
    server.request(serde_json::json!({"jsonrpc":"2.0","id":0,"method":"initialize"}));
    server.request(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    server.request(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}})).unwrap()["result"].clone()
}

#[test]
fn qualified_rust_method_selects_its_impl_instead_of_same_named_methods() {
    let dir = tempfile::tempdir().unwrap();
    common::write_file(dir.path(), "store.rs", "struct Alpha;\nimpl Alpha { pub fn load(&self) -> &'static str { \"alpha implementation\" } }\nstruct Beta;\nimpl Beta { pub fn load(&self) -> &'static str { \"beta implementation\" } }\n");
    let result = call_tool(
        dir.path(),
        "get_symbol",
        serde_json::json!({"paths":["store.rs"],"symbol":"Alpha::load","max_tokens":2000}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let content = result["structuredContent"]["files"][0]["content"]
        .as_str()
        .unwrap();
    assert!(content.contains("alpha implementation"), "{content}");
    assert!(!content.contains("beta implementation"), "{content}");
}

#[test]
fn qualified_rust_method_keeps_free_function_resolution_outside_impl_namespace() {
    let dir = tempfile::tempdir().unwrap();
    common::write_file(dir.path(), "store.rs", "fn helper() -> &'static str { \"free helper\" }\nstruct Alpha;\nimpl Alpha {\n    fn helper() -> &'static str { \"member helper\" }\n    fn load() -> &'static str { helper() }\n}\n");
    let result = call_tool(
        dir.path(),
        "get_symbol",
        serde_json::json!({"paths":["store.rs"],"symbol":"Alpha::load","max_tokens":2000}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let content = result["structuredContent"]["files"][0]["content"]
        .as_str()
        .unwrap();
    assert!(content.contains("free helper"), "{content}");
    assert!(!content.contains("member helper"), "{content}");
}

#[test]
fn supporting_dependencies_do_not_displace_query_matches_in_search_budget() {
    let dir = tempfile::tempdir().unwrap();
    common::write_file(
        dir.path(),
        "entry.py",
        "import noise\n\ndef cache_snapshot():\n    return noise.helper()\n",
    );
    common::write_file(
        dir.path(),
        "state.py",
        "def snapshot():\n    return \"state evidence\"\n",
    );
    let mut noise = "def helper():\n    return 0\n".to_string();
    for n in 0..10 {
        noise.push_str(&format!("\ndef utility_{n}():\n    return {n}\n"));
    }
    common::write_file(dir.path(), "noise.py", &noise);
    let result = call_tool(
        dir.path(),
        "search",
        serde_json::json!({"paths":["entry.py","state.py"],"query":"cache snapshot","max_tokens":1200}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let packet = &result["structuredContent"];
    let paths: Vec<_> = packet["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["path"].as_str())
        .collect();
    assert!(
        paths.contains(&"entry.py") && paths.contains(&"state.py"),
        "{packet}"
    );
    assert!(packet["metadata"]["estimated_tokens"].as_u64().unwrap() <= 1200);
}

#[test]
fn focused_symbols_keep_ancestor_imports_without_unrelated_nested_imports() {
    let dir = tempfile::tempdir().unwrap();
    common::write_file(dir.path(), "scopes.rs", "use std::path::Path;\npub fn selected() { let _ = Path::new(\".\"); }\nmod tests {\n    use std::thread;\n    fn inner() { let _ = thread::current(); }\n}\n");
    let outer = call_tool(
        dir.path(),
        "get_symbol",
        serde_json::json!({"paths":["scopes.rs"],"symbol":"selected","max_tokens":2000}),
    );
    assert_eq!(outer["isError"], false, "{outer}");
    let content = outer["structuredContent"]["files"][0]["content"]
        .as_str()
        .unwrap();
    assert!(content.contains("use std::path::Path;"), "{content}");
    assert!(!content.contains("use std::thread;"), "{content}");
    let inner = call_tool(
        dir.path(),
        "get_symbol",
        serde_json::json!({"paths":["scopes.rs"],"symbol":"inner","max_tokens":2000}),
    );
    assert_eq!(inner["isError"], false, "{inner}");
    assert!(inner["structuredContent"]["files"][0]["content"]
        .as_str()
        .unwrap()
        .contains("use std::thread;"));
}

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
