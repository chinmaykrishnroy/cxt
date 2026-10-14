use crate::{cli::Args, engine, security, symbols::ContextLevel};
use clap::{Parser, ValueEnum};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::path::{Component, Path, PathBuf};

const MAX_FRAME: usize = 1024 * 1024;
const MAX_RESPONSE: usize = 1024 * 1024;
const TOOLS: &[&str] = &[
    "get_context",
    "search",
    "trace",
    "get_symbol",
    "get_diff_context",
    "get_related_tests",
    "get_repo_map",
];

pub struct Server {
    root: PathBuf,
    started: bool,
    initialized: bool,
}
impl Server {
    pub fn new(root: &Path) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|_| "MCP workspace does not exist")?;
        if !root.is_dir() {
            return Err("MCP requires one directory workspace".into());
        }
        crate::scan::ScanLimits::default().guard(&root)?;
        Ok(Self {
            root,
            started: false,
            initialized: false,
        })
    }
    pub fn request(&mut self, request: Value) -> Option<Value> {
        if !request.is_object() || !request.get("method").is_some_and(Value::is_string) {
            return Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"invalid JSON-RPC request"}}),
            );
        }
        let id = request.get("id").cloned();
        if id
            .as_ref()
            .is_some_and(|id| !id.is_string() && !id.is_number() && !id.is_null())
        {
            return Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"invalid request id"}}),
            );
        }
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        if request["jsonrpc"] != "2.0" {
            return id.map(|id| json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":"invalid JSON-RPC request"}}));
        }
        if method == "notifications/initialized" {
            if self.started {
                self.initialized = true;
            }
            return None;
        }
        let id = id?;
        let result = match method {
            "initialize" => {
                self.started = true;
                let requested = request["params"]["protocolVersion"]
                    .as_str()
                    .unwrap_or("2025-03-26");
                let version = if requested == "2025-06-18" {
                    requested
                } else {
                    "2025-03-26"
                };
                Ok(
                    json!({"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"cxt","version":env!("CARGO_PKG_VERSION")},"instructions":"Read-only local source context; spans and imports are static approximations, not proof of completeness."}),
                )
            }
            "ping" => Ok(json!({})),
            "tools/list" if self.initialized => Ok(
                json!({"tools":TOOLS.iter().map(|name| json!({"name":name,"description":format!("Read-only {name}: sanitized context and original-source spans in the configured workspace"),"inputSchema":{"type":"object","properties":{"paths":{"type":"array","items":{"type":"string"}},"query":{"type":"string","maxLength":1024},"symbol":{"type":"string"},"context":{"type":"string","enum":["complete","map","focused","adaptive"]},"max_tokens":{"type":"integer","minimum":1,"maximum":32000},"git":{"type":"string"}},"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}})).collect::<Vec<_>>()}),
            ),
            "tools/call" if self.initialized => {
                let params = &request["params"];
                match self.call(
                    params["name"].as_str().unwrap_or(""),
                    params.get("arguments").unwrap_or(&Value::Null),
                ) {
                    Ok(packet) => Ok(
                        json!({"content":[{"type":"text","text":packet.to_string()}],"structuredContent":packet,"isError":false}),
                    ),
                    Err(message) => {
                        Ok(json!({"content":[{"type":"text","text":message}],"isError":true}))
                    }
                }
            }
            _ => Err("unsupported method or initialization required"),
        };
        Some(match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
            Err(message) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":message}})
            }
        })
    }
    fn call(&self, name: &str, input: &Value) -> Result<Value, String> {
        if !TOOLS.contains(&name) {
            return Err("unknown tool".into());
        }
        if let Some(map) = input.as_object() {
            if map.keys().any(|k| {
                !["paths", "query", "symbol", "context", "max_tokens", "git"].contains(&k.as_str())
            }) {
                return Err("unsupported tool argument".into());
            }
            for key in ["query", "symbol", "context", "git"] {
                if map.get(key).is_some_and(|v| !v.is_string()) {
                    return Err(format!("{key} must be a string"));
                }
            }
            if map
                .get("max_tokens")
                .is_some_and(|v| !v.as_u64().is_some_and(|n| (1..=32000).contains(&n)))
            {
                return Err("max_tokens must be between 1 and 32000".into());
            }
        } else if !input.is_null() {
            return Err("arguments must be an object".into());
        }
        let mut args = Args::parse_from(["cxt"]).normalized();
        args.mcp = true;
        args.paths = vec![self.root.clone()];
        args.workspace_root = Some(self.root.clone());
        args.no_cache = true;
        args.stdout = true;
        args.ci = true;
        args.format = Some(crate::output::OutputFormat::Json);
        args.model = Some("cl100k".into());
        args.max_tokens =
            Some(input["max_tokens"].as_u64().unwrap_or(8000).clamp(1, 32000) as usize);
        args.scan_max_entries = Some(10_000);
        args.scan_max_mb = Some(16);
        args.scan_max_depth = Some(24);
        args.scan_timeout = Some(10);
        args.secrets = Some(security::SecretPolicy::Redact);
        args.manifest = true;
        args.query = input["query"].as_str().map(str::to_string);
        if args.query.as_ref().is_some_and(|q| q.len() > 1024) {
            return Err("query exceeds limit".into());
        }
        args.symbol = input["symbol"].as_str().map(str::to_string);
        if args
            .symbol
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 1024)
        {
            return Err("symbol must contain 1–1024 UTF-8 bytes".into());
        }
        if let Some(context) = input["context"].as_str() {
            args.context = ContextLevel::from_str(context, false)?;
        } else {
            args.context = ContextLevel::Adaptive;
        }
        if let Some(paths) = input.get("paths") {
            let paths = paths.as_array().ok_or("paths must be an array")?;
            if paths.is_empty() || paths.len() > 32 {
                return Err("supply 1–32 workspace-relative paths".into());
            }
            args.paths = paths
                .iter()
                .map(|p| {
                    let path = Path::new(p.as_str().ok_or("path must be a string")?);
                    if path.is_absolute()
                        || path
                            .components()
                            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
                    {
                        return Err("paths must remain relative to the authorized workspace".into());
                    }
                    crate::scope::check(Some(&self.root), &self.root.join(path))
                })
                .collect::<Result<_, String>>()?;
        }
        match name {
            "search" => {
                if args.query.is_none() {
                    return Err("search requires query".into());
                }
                args.context = ContextLevel::Map;
            }
            "trace" => {
                if input.get("paths").is_none() || args.paths.iter().any(|p| !p.is_file()) {
                    return Err("trace requires explicit workspace-relative entry files".into());
                }
                args.trace = true;
            }
            "get_symbol" => {
                if args.symbol.is_none() {
                    return Err("get_symbol requires symbol".into());
                }
                args.context = ContextLevel::Focused;
            }
            "get_repo_map" => args.context = ContextLevel::Map,
            "get_related_tests" if input.get("paths").is_some() => {
                let expanded = crate::impact::expand(&self.root, &args.paths, &args, &self.root)?;
                let tests: Vec<_> = expanded
                    .candidates
                    .into_iter()
                    .filter(|(_, reason)| reason == "related-test")
                    .map(|(path, _)| path)
                    .collect();
                if tests.is_empty() {
                    return Ok(
                        json!({"schema":"cxt.context.v1","files":[],"warnings":["No related tests found by static naming/import heuristics"]}),
                    );
                }
                args.paths = tests;
            }
            "get_diff_context" | "get_related_tests" => {
                let mode = input["git"].as_str().unwrap_or("working");
                if mode.starts_with('-')
                    || mode.len() > 128
                    || mode.chars().any(|c| c.is_whitespace() || c.is_control())
                {
                    return Err("invalid Git selection".into());
                }
                let repository = crate::git::repository_root(&self.root)?;
                crate::scope::check(Some(&self.root), &repository)?;
                args.git = Some(mode.into());
                args.impact = name == "get_related_tests";
            }
            _ => {}
        }
        let result = engine::run(&args, &self.root).map_err(|e| e.to_string())?;
        if result.rendered.len() > MAX_RESPONSE / 3 {
            return Err("context exceeds MCP response limit; narrow the request".into());
        }
        serde_json::from_str(&result.rendered).map_err(|_| "invalid context serialization".into())
    }
}

pub fn serve(args: &Args) -> Result<(), String> {
    if args.paths.len() != 1 {
        return Err("--mcp requires exactly one workspace directory".into());
    }
    let mut server = Server::new(&args.paths[0])?;
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut bytes = Vec::new();
        let count = (&mut input)
            .take((MAX_FRAME + 1) as u64)
            .read_until(b'\n', &mut bytes)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        let reply = if bytes.len() > MAX_FRAME {
            while !bytes.ends_with(b"\n") {
                bytes.clear();
                if (&mut input)
                    .take((MAX_FRAME + 1) as u64)
                    .read_until(b'\n', &mut bytes)
                    .map_err(|e| e.to_string())?
                    == 0
                {
                    break;
                }
            }
            Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"request exceeded frame limit"}}),
            )
        } else {
            match serde_json::from_slice(&bytes) {
                Ok(request) => server.request(request),
                Err(_) => Some(
                    json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"invalid JSON"}}),
                ),
            }
        };
        if let Some(reply) = reply {
            let encoded = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
            if encoded.len() > MAX_RESPONSE {
                serde_json::to_writer(&mut output, &json!({"jsonrpc":"2.0","id":reply["id"],"error":{"code":-32603,"message":"response exceeded limit; narrow request"}})).map_err(|e| e.to_string())?;
            } else {
                output.write_all(&encoded).map_err(|e| e.to_string())?;
            }
            output.write_all(b"\n").map_err(|e| e.to_string())?;
            output.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
