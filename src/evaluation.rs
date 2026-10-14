use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct Corpus {
    schema: String,
    dataset_kind: String,
    budgets: Vec<usize>,
    tasks: Vec<Task>,
    #[serde(default = "default_rankers")]
    rankers: Vec<String>,
    #[serde(default = "default_contexts")]
    contexts: Vec<String>,
    #[serde(default)]
    max_files: Option<usize>,
    #[serde(default = "default_expansion")]
    query_expansion: Vec<bool>,
}
fn default_expansion() -> Vec<bool> {
    vec![true]
}
fn default_rankers() -> Vec<String> {
    vec!["hybrid".into()]
}
fn default_contexts() -> Vec<String> {
    ["complete", "focused", "adaptive"]
        .map(str::to_string)
        .to_vec()
}
#[derive(Deserialize, serde::Serialize)]
struct Task {
    id: String,
    root: PathBuf,
    query: String,
    required_files: Vec<String>,
    required_symbols: Vec<Label>,
}
#[derive(Deserialize, serde::Serialize)]
struct Label {
    path: String,
    name: String,
    start_line: usize,
    end_line: usize,
    #[serde(default)]
    source_sha256: Option<String>,
}

fn normalized(path: &str) -> String {
    path.replace('\\', "/")
}

struct InvocationFailure {
    message: String,
    kind: &'static str,
    exit_code: Option<i32>,
    elapsed_ms: Option<f64>,
    peak_memory_bytes: Option<u64>,
}

impl From<&str> for InvocationFailure {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
            kind: "invocation_error",
            exit_code: None,
            elapsed_ms: None,
            peak_memory_bytes: None,
        }
    }
}

// Classify only; never retain raw stderr, which can contain source or credentials.
fn failure_kind(code: Option<i32>, stderr: &str) -> &'static str {
    if code == Some(4) {
        if stderr.contains("secret policy 'fail'") {
            "secret_policy"
        } else {
            "security_scanner"
        }
    } else if stderr.contains("Scan stopped: time limit reached") {
        "scan_timeout"
    } else if stderr.contains("Scan stopped:") {
        "scan_limit"
    } else if stderr.contains("query matched no files") {
        "no_query_match"
    } else if stderr.contains("token budget")
        || stderr.contains("no relevant implementation span fits")
        || stderr.contains("metadata/output overhead exceeds")
        || stderr.contains("no selected file fit after including rendered metadata overhead")
    {
        "token_budget"
    } else {
        "cli_error"
    }
}

#[cfg(windows)]
fn peak_memory(child: &Child) -> Option<u64> {
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    struct Counters {
        cb: u32,
        faults: u32,
        sizes: [usize; 8],
    }
    #[link(name = "psapi")]
    extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut Counters,
            size: u32,
        ) -> i32;
    }
    let mut counters = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        faults: 0,
        sizes: [0; 8],
    };
    // The live Child owns this process handle; the repr(C) buffer matches PROCESS_MEMORY_COUNTERS.
    let ok = unsafe {
        GetProcessMemoryInfo(
            child.as_raw_handle(),
            &mut counters,
            std::mem::size_of::<Counters>() as u32,
        )
    };
    (ok != 0).then_some(counters.sizes[0] as u64)
}
#[cfg(target_os = "linux")]
fn peak_memory(child: &Child) -> Option<u64> {
    std::fs::read_to_string(format!("/proc/{}/status", child.id()))
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")
                .and_then(|s| s.split_whitespace().next())
                .and_then(|n| n.parse::<u64>().ok())
                .map(|n| n * 1024)
        })
}
#[cfg(not(any(windows, target_os = "linux")))]
fn peak_memory(_: &Child) -> Option<u64> {
    None
}

fn execute(
    executable: &Path,
    root: &Path,
    arguments: &[String],
) -> Result<(Value, String, f64, Option<u64>), InvocationFailure> {
    let started = Instant::now();
    let mut child = Command::new(executable)
        .current_dir(root)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "evaluation executable could not start")?;
    let stdout = child.stdout.take().ok_or("missing child stdout")?;
    let mut stderr = child.stderr.take().ok_or("missing child stderr")?;
    let error_reader = std::thread::spawn(move || {
        let mut retained = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(length) = stderr.read(&mut buffer) {
            if length == 0 {
                break;
            }
            let keep = length.min((64usize * 1024).saturating_sub(retained.len()));
            retained.extend_from_slice(&buffer[..keep]);
        }
        retained
    });
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let mut peak = None;
    let mut timed_out = false;
    let status = loop {
        if let Some(value) = peak_memory(&child) {
            peak = Some(peak.unwrap_or(0).max(value));
        }
        match child
            .try_wait()
            .map_err(|_| "evaluation child status failed")?
        {
            Some(status) => break status,
            None if started.elapsed() > Duration::from_secs(60) => {
                timed_out = true;
                let _ = child.kill();
                break child
                    .wait()
                    .map_err(|_| "evaluation child termination failed")?;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "evaluation output reader failed")?
        .map_err(|_| "evaluation output read failed")?;
    let errors = error_reader
        .join()
        .map_err(|_| "evaluation error reader failed")?;
    if timed_out {
        return Err(InvocationFailure {
            message: "evaluation invocation exceeded 60 seconds".into(),
            kind: "invocation_timeout",
            exit_code: status.code(),
            elapsed_ms: Some(started.elapsed().as_secs_f64() * 1000.0),
            peak_memory_bytes: peak,
        });
    }
    if !status.success() {
        return Err(InvocationFailure {
            message: format!(
                "CLI exited with code {:?}; no context accepted",
                status.code()
            ),
            kind: failure_kind(status.code(), &String::from_utf8_lossy(&errors)),
            exit_code: status.code(),
            elapsed_ms: Some(started.elapsed().as_secs_f64() * 1000.0),
            peak_memory_bytes: peak,
        });
    }
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("evaluation output exceeded limit".into());
    }
    let document =
        serde_json::from_slice(&bytes).map_err(|_| "CLI did not emit a JSON context document")?;
    let rendered = String::from_utf8(bytes).map_err(|_| "CLI emitted invalid UTF-8")?;
    Ok((
        document,
        rendered,
        started.elapsed().as_secs_f64() * 1000.0,
        peak,
    ))
}

pub fn evaluate(path: &Path, executable: &Path, baseline: Option<&Path>) -> Result<Value, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| "corpus file is unavailable")?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "corpus read failed")?;
    if bytes.len() > 1024 * 1024 {
        return Err("corpus exceeds 1 MiB".into());
    }
    let corpus: Corpus =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid evaluation corpus: {e}"))?;
    if corpus.query_expansion.is_empty()
        || corpus.query_expansion.len() > 2
        || corpus.rankers.is_empty()
        || corpus.rankers.len() > 4
        || corpus
            .rankers
            .iter()
            .any(|r| !["hybrid", "lexical", "symbols", "legacy"].contains(&r.as_str()))
        || corpus.contexts.is_empty()
        || corpus.contexts.len() > 4
        || corpus
            .contexts
            .iter()
            .any(|r| !["complete", "map", "focused", "adaptive"].contains(&r.as_str()))
    {
        return Err("invalid evaluation strategy modes".into());
    }
    if corpus.max_files.is_some_and(|n| n == 0 || n > 1000)
        || corpus.schema != "cxt.eval.v1"
        || corpus.tasks.is_empty()
        || corpus.tasks.len() > 100
        || corpus.budgets.is_empty()
        || corpus.budgets.len() > 8
        || corpus.budgets.iter().any(|b| *b == 0 || *b > 100_000)
    {
        return Err("invalid corpus schema/task/budget limits".into());
    }
    let modes = corpus.contexts.len() * corpus.rankers.len() * corpus.query_expansion.len()
        + usize::from(baseline.is_some());
    if corpus.tasks.len() * corpus.budgets.len() * modes * 2 > 5000 {
        return Err("corpus exceeds 5000 invocation limit; split the experiment".into());
    }
    let base = path
        .canonicalize()
        .map_err(|_| "corpus path is unavailable")?
        .parent()
        .ok_or("corpus directory missing")?
        .to_path_buf();
    let executable = executable
        .canonicalize()
        .map_err(|_| "current executable unavailable")?;
    let baseline = baseline
        .map(|p| {
            p.canonicalize()
                .map_err(|_| "baseline executable unavailable")
        })
        .transpose()?;
    let counter = crate::token::TokenCounter::for_model("cl100k");
    let mut runs = Vec::new();
    let mut fingerprints = std::collections::BTreeMap::new();
    let mut labels = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for task in corpus.tasks {
        labels
            .push(serde_json::to_value(&task).map_err(|_| "task labels could not be serialized")?);
        if !ids.insert(task.id.clone()) || task.query.len() > 4096 || task.required_files.is_empty()
        {
            return Err("invalid or duplicate task labels".into());
        }
        let root = crate::scope::check(Some(&base), &base.join(&task.root))?;
        crate::scan::ScanLimits::default().guard(&root)?;
        let mut label_costs = Vec::new();
        let mut label_texts = Vec::new();
        for label in &task.required_symbols {
            if label.start_line == 0 || label.end_line < label.start_line {
                return Err("invalid required symbol span".into());
            }
            let source_path = crate::scope::check(Some(&root), &root.join(&label.path))?;
            let mut text = String::new();
            std::fs::File::open(&source_path)
                .map_err(|_| "required source missing")?
                .take(1024 * 1024 + 1)
                .read_to_string(&mut text)
                .map_err(|_| "required source is not UTF-8")?;
            if text.len() > 1024 * 1024 {
                return Err("required source label exceeds 1 MiB".into());
            }
            let lines: Vec<_> = text.split_inclusive('\n').collect();
            let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
            if label.end_line > lines.len()
                || label.name.is_empty()
                || label
                    .source_sha256
                    .as_ref()
                    .is_some_and(|expected| expected != &hash)
            {
                return Err("symbol label does not match the source revision".into());
            }
            let span = lines[label.start_line - 1..label.end_line]
                .concat()
                .trim_start_matches([' ', '\t'])
                .trim_end_matches(['\r', '\n'])
                .to_string();
            label_costs.push(counter.count(&span));
            label_texts.push(span);
            fingerprints.insert(
                format!("{}/{}", task.id, label.path),
                format!("{:x}", Sha256::digest(text.as_bytes())),
            );
        }
        for file in &task.required_files {
            crate::scope::check(Some(&root), &root.join(file))?;
        }
        for budget in &corpus.budgets {
            let mut modes = Vec::new();
            for context in &corpus.contexts {
                for ranker in &corpus.rankers {
                    for expand in &corpus.query_expansion {
                        modes.push((
                            "current",
                            context.as_str(),
                            ranker.as_str(),
                            *expand,
                            &executable,
                        ));
                    }
                }
            }
            if let Some(baseline) = &baseline {
                modes.push(("baseline", "complete", "legacy", true, baseline));
            }
            for (tool, context, ranker, expand, executable) in modes {
                let cache = tempfile::tempdir().map_err(|_| "evaluation cache unavailable")?;
                for phase in ["cold", "repeat"] {
                    let mut arguments = vec![
                        ".".into(),
                        "--query".into(),
                        task.query.clone(),
                        "--max-tokens".into(),
                        budget.to_string(),
                        "--model".into(),
                        "cl100k".into(),
                        "--format".into(),
                        "json".into(),
                        "--path-style".into(),
                        "relative".into(),
                        "--no-minify".into(),
                        "--secrets".into(),
                        "fail".into(),
                        "--stdout".into(),
                    ];
                    if let Some(max_files) = corpus.max_files {
                        arguments.extend(["--max-files".into(), max_files.to_string()]);
                    }
                    if tool == "current" {
                        if !expand {
                            arguments.push("--no-query-expansion".into());
                        }
                        arguments.extend([
                            "--ranker".into(),
                            ranker.into(),
                            "--context".into(),
                            context.into(),
                            "--cache-dir".into(),
                            cache.path().display().to_string(),
                            "--workspace-root".into(),
                            root.display().to_string(),
                        ]);
                    }
                    let mut record = json!({"task":task.id,"query":task.query,"launch_directory":root,"arguments":arguments,"tool":tool,"executable":executable.display().to_string(),"context":context,"ranker":ranker,"query_expansion":expand,"phase":phase,"budget":budget,"file_recall":0.0,"symbol_recall":0.0,"success":false});
                    match execute(executable, &root, &arguments) {
                        Err(error) => {
                            record["error"] = json!(error.message);
                            record["failure_kind"] = json!(error.kind);
                            record["exit_code"] = json!(error.exit_code);
                            record["elapsed_ms"] = json!(error.elapsed_ms);
                            record["peak_memory_bytes"] = json!(error.peak_memory_bytes);
                        }
                        Ok((packet, rendered, elapsed, peak)) => {
                            let tokens = counter.count(&rendered);
                            let accepted = tokens <= *budget;
                            let files = packet["files"]
                                .as_array()
                                .ok_or("packet has no files array")?;
                            let hits = task
                                .required_files
                                .iter()
                                .filter(|required| {
                                    files.iter().any(|f| {
                                        f["path"]
                                            .as_str()
                                            .is_some_and(|p| normalized(p) == normalized(required))
                                    })
                                })
                                .count();
                            let covered: Vec<bool> = task
                                .required_symbols
                                .iter()
                                .zip(&label_texts)
                                .map(|(label, original_span)| {
                                    files.iter().any(|f| {
                                        if !f["path"].as_str().is_some_and(|p| {
                                            normalized(p) == normalized(&label.path)
                                        }) || f["representation"] == "map"
                                        {
                                            return false;
                                        }
                                        let source_present = f["content"]
                                            .as_str()
                                            .is_some_and(|s| s.contains(original_span));
                                        if !source_present {
                                            return false;
                                        }
                                        match f["spans"].as_array() {
                                            Some(spans) if !spans.is_empty() => {
                                                spans.iter().any(|s| {
                                                    s["start_line"].as_u64().is_some_and(|n| {
                                                        n as usize <= label.start_line
                                                    }) && s["end_line"].as_u64().is_some_and(|n| {
                                                        n as usize >= label.end_line
                                                    })
                                                })
                                            }
                                            _ => true,
                                        }
                                    })
                                })
                                .collect();
                            let symbols = covered.iter().filter(|hit| **hit).count();
                            let label_tokens: usize = label_costs
                                .iter()
                                .zip(&covered)
                                .filter(|(_, covered)| **covered)
                                .map(|(cost, _)| *cost)
                                .sum();
                            record["success"] = json!(accepted);
                            record["final_tokens"] = json!(tokens);
                            record["elapsed_ms"] = json!(elapsed);
                            record["peak_memory_bytes"] = json!(peak);
                            record["file_recall"] = json!(if accepted {
                                hits as f64 / task.required_files.len() as f64
                            } else {
                                0.0
                            });
                            record["symbol_recall"] =
                                json!(if accepted && !task.required_symbols.is_empty() {
                                    symbols as f64 / task.required_symbols.len() as f64
                                } else {
                                    0.0
                                });
                            record["required_source_token_share_proxy"] =
                                json!((label_tokens as f64 / tokens.max(1) as f64).min(1.0));
                            record["accounting"] = packet["metadata"]["token_accounting"].clone();
                            if !accepted {
                                record["error"] = json!(
                                    "final packet exceeded independently counted exact budget"
                                );
                            }
                            let mut stable = packet;
                            if let Some(metadata) = stable["metadata"].as_object_mut() {
                                metadata.remove("generated_at_unix");
                            }
                            record["selection_sha256"] = json!(format!(
                                "{:x}",
                                Sha256::digest(stable.to_string().as_bytes())
                            ));
                        }
                    }
                    runs.push(record);
                }
            }
        }
    }
    Ok(
        json!({"schema":"cxt.eval.report.v1","dataset_kind":corpus.dataset_kind,"tool_version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"tokenizer":"cl100k_base (ordinary text; packet cost only)","downstream_task_success":"not_measured","precision_metric":"required_source_token_share_proxy is not measured relevant-token precision; labels may omit supporting source","memory_scope":"per-child process peak working set where supported; null otherwise","cache_scope":"repeat phase shares current-tool metadata cache; baseline may rescan; CLI startup/tokenizer initialization included","labels":labels,"source_fingerprints":fingerprints,"runs":runs}),
    )
}
