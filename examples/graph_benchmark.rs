use clap::Parser;
use cxt_core::{cli::Args, review::Workspace};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::Instant;

fn source(index: usize, modules: usize, functions: usize) -> String {
    let mut text = format!(
        "from module{:04} import helper_0000\n\n",
        (index + 1) % modules
    );
    for function in 0..functions {
        text.push_str(&format!(
            "def helper_{function:04}(value):\n    return value * 3 + {function}\n\n"
        ));
    }
    text
}

fn measure(root: &Path, args: &Args, phase: &str) -> Result<Value, String> {
    let started = Instant::now();
    let workspace = Workspace::load(args, root)?;
    Ok(
        json!({"phase":phase,"elapsed_ms":started.elapsed().as_secs_f64()*1000.0,"source_nodes":workspace.dependencies.len(),"graph":workspace.graph_stats}),
    )
}

fn run() -> Result<Value, String> {
    let inputs: Vec<_> = std::env::args().skip(1).collect();
    let modules = inputs
        .first()
        .map(|n| n.parse::<usize>())
        .transpose()
        .map_err(|_| "module count must be numeric")?
        .unwrap_or(500);
    let functions = inputs
        .get(1)
        .map(|n| n.parse::<usize>())
        .transpose()
        .map_err(|_| "function count must be numeric")?
        .unwrap_or(200);
    if !(2..=2000).contains(&modules)
        || !(1..=1000).contains(&functions)
        || modules * functions > 500_000
    {
        return Err("use 2–2000 modules, 1–1000 functions, at most 500000 functions total".into());
    }
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let root = directory.path();
    fs::write(
        root.join("pyproject.toml"),
        "[project]\nname='cxt-synthetic-graph'\nversion='1.0.0'\n",
    )
    .map_err(|e| e.to_string())?;
    for module in 0..modules {
        fs::write(
            root.join(format!("module{module:04}.py")),
            source(module, modules, functions),
        )
        .map_err(|e| e.to_string())?;
    }
    let args = Args::parse_from([
        "cxt",
        "module0000.py",
        "--cache-dir",
        ".cxt/cache",
        "--workspace-root",
        ".",
        "--scan-timeout",
        "180",
        "--scan-max-mb",
        "200",
    ])
    .normalized();
    let mut phases = vec![
        measure(root, &args, "cold")?,
        measure(root, &args, "repeat")?,
    ];
    if phases[1]["graph"]["reused"] != modules {
        return Err("repeat did not reuse every unchanged source node".into());
    }
    let changed = format!(
        "{}\n# source edit\n",
        source(modules - 1, modules, functions)
    );
    fs::write(root.join(format!("module{:04}.py", modules - 1)), changed)
        .map_err(|e| e.to_string())?;
    phases.push(measure(root, &args, "one-source-edit")?);
    if phases[2]["graph"]["rebuilt"] != 1 {
        return Err("single-source edit did not rebuild exactly one node".into());
    }
    fs::rename(root.join("module0001.py"), root.join("renamed.py")).map_err(|e| e.to_string())?;
    fs::write(
        root.join("module0000.py"),
        source(0, modules, functions).replace("module0001", "renamed"),
    )
    .map_err(|e| e.to_string())?;
    phases.push(measure(root, &args, "rename-and-import-edit")?);
    let mut uncached = args.clone();
    uncached.no_cache = true;
    phases.push(measure(root, &uncached, "cache-disabled")?);
    Ok(
        json!({"schema":"cxt.graph-benchmark.v1","dataset":"generated Python modules, not a public monorepo","modules":modules,"functions_per_module":functions,"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"scope":"full Workspace load including fresh source enumeration/read/hash, metadata validation and graph assembly; not zero-read querying","phases":phases}),
    )
}

fn main() {
    match run() {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
        Err(error) => {
            eprintln!("graph benchmark: {error}");
            std::process::exit(1);
        }
    }
}
