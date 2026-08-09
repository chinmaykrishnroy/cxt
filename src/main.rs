use arboard::Clipboard;
use clap::Parser;
use colored::Colorize;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use minifier::{count_tokens, minify_code, redact_secrets, strip_comments};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracer::get_language;
use utils::is_binary_file;

mod config;
mod minifier;
mod resolver;
mod shimmer;
mod tracer;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about = "Blazing fast, smart context packer for LLMs", long_about = None)]
struct Args {
    /// Target path to scan (or entry file for AST tracing)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Only pack files with uncommitted Git changes
    #[arg(short, long)]
    diff: bool,

    /// Minify code (strip comments and extra whitespace) to save LLM tokens
    #[arg(short, long)]
    minify: bool,

    /// Enable AST tracing to only pack dependencies of a specific file
    #[arg(short, long)]
    trace: bool,

    /// Write output to a file instead of copying to the clipboard
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Bypass file size and lock/minified-file safety limits
    #[arg(long)]
    no_limit: bool,

    /// Show selected files without generating or copying output
    #[arg(long)]
    dry_run: bool,
}

const MAX_FILE_SIZE: u64 = 1024 * 1024; // 1 MB

fn get_git_modified_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .output()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            files.push(PathBuf::from(line));
        }
    }

    if let Ok(output) = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            files.push(PathBuf::from(line));
        }
    }

    files
}

fn build_ignore_globset(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        let normalized = config::normalize_ignore_pattern(p);
        if let Ok(g) = Glob::new(&normalized) {
            builder.add(g);
        }
    }
    builder
        .build()
        .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap())
}

fn build_ignore_overrides(patterns: &[String], root: &Path) -> ignore::overrides::Override {
    let mut builder = OverrideBuilder::new(root);
    for p in patterns {
        let normalized = config::normalize_ignore_pattern(p);
        let _ = builder.add(&format!("!{}", normalized));
    }
    builder
        .build()
        .unwrap_or_else(|_| OverrideBuilder::new(root).build().unwrap())
}

fn should_skip_file(path: &Path, no_limit: bool, ignore_globset: &GlobSet) -> bool {
    if !no_limit {
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() > MAX_FILE_SIZE {
                return true;
            }
        }

        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        if file_name.ends_with(".lock")
            || file_name.ends_with("-lock.json")
            || ext == "svg"
            || ext == "png"
            || ext == "jpg"
            || ext == "woff2"
            || stem.ends_with(".min")
        {
            return true;
        }
    }

    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if ignore_globset.is_match(path) || ignore_globset.is_match(Path::new(file_name)) {
        return true;
    }

    false
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green.bold} {msg}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

fn progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green.bold} [{pos}/{len}] {msg}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

const SHIMMER_INTERVAL_MS: u64 = 80;

/// Spawns a background thread that applies the shimmer effect to `prefix` at a fixed rate,
/// while appending `suffix` unchanged. Callers should set `done` and join the handle before
/// calling `pb.finish_and_clear()`.
fn spawn_shimmer(
    pb: ProgressBar,
    prefix: String,
    initial_suffix: String,
) -> (Arc<Mutex<String>>, Arc<AtomicBool>, JoinHandle<()>) {
    let suffix = Arc::new(Mutex::new(initial_suffix));
    let done = Arc::new(AtomicBool::new(false));
    let pb_clone = pb.clone();
    let suffix_clone = Arc::clone(&suffix);
    let done_clone = Arc::clone(&done);

    let handle = thread::spawn(move || {
        let mut tick: u64 = 0;
        while !done_clone.load(Ordering::Relaxed) {
            if let Ok(s) = suffix_clone.lock() {
                let rendered = format!("{}{}", shimmer::apply_shimmer(&prefix, tick), s.as_str());
                pb_clone.set_message(rendered);
            }
            tick = tick.wrapping_add(1);
            thread::sleep(Duration::from_millis(SHIMMER_INTERVAL_MS));
        }
    });

    (suffix, done, handle)
}

fn main() {
    let args = Args::parse();
    let config = config::Config::load();
    let minify = args.minify || config.default_minify;
    let ignore_globset = build_ignore_globset(&config.global_ignore);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!(
            "{}",
            "\n[!] Received Ctrl+C. Halting immediately...".yellow()
        );
        r.store(false, Ordering::SeqCst);
        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    let max_output_bytes = config
        .max_output_mb
        .map(|m| m * 1024 * 1024)
        .unwrap_or(100 * 1024 * 1024);

    let mut final_output = String::with_capacity(2 * 1024 * 1024);
    final_output.push_str("# Codebase Context\n\n");

    let target_files: Vec<PathBuf>;

    if args.diff {
        println!("{}", "[+] Mode: Git Diff (Only changed files)".green());
        target_files = get_git_modified_files();
    } else if args.trace && args.path.is_file() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(spinner_style());
        pb.enable_steady_tick(Duration::from_millis(100));

        let (trace_suffix, trace_done, trace_handle) = spawn_shimmer(
            pb.clone(),
            format!("Tracing dependencies for {}", args.path.display()),
            String::new(),
        );

        let mut trace_count = 0;
        target_files = tracer::trace_dependencies(&args.path, move |p| {
            trace_count += 1;
            if trace_count % 10 == 0 {
                if let Ok(mut s) = trace_suffix.lock() {
                    *s = format!("\x1b[38;2;85;85;85m -> {}\x1b[0m", p.display());
                }
            }
        });

        trace_done.store(true, Ordering::Relaxed);
        trace_handle.join().unwrap();
        pb.finish_and_clear();
        println!(
            "{} Smart AST Trace ({})",
            "[+]".green(),
            args.path.display().to_string().cyan()
        );
    } else {
        let mut walker = WalkBuilder::new(&args.path);
        walker.hidden(false);
        if !config.global_ignore.is_empty() {
            walker.overrides(build_ignore_overrides(&config.global_ignore, &args.path));
        }
        let pb = ProgressBar::new_spinner();
        pb.set_style(spinner_style());
        pb.enable_steady_tick(Duration::from_millis(100));

        let (scan_suffix, scan_done, scan_handle) = spawn_shimmer(
            pb.clone(),
            format!("Scanning directory {}", args.path.display()),
            String::new(),
        );

        let mut files = Vec::new();
        let mut scan_count = 0;
        for entry in walker.build().flatten() {
            if !running.load(Ordering::SeqCst) {
                println!("{}", "[!] Aborted by user.".yellow());
                break;
            }

            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                let path = entry.path();
                if path.components().any(|c| {
                    let name = c.as_os_str().to_string_lossy();
                    name == ".git" || name == "target"
                }) {
                    continue;
                }
                files.push(path.to_path_buf());
                scan_count += 1;
                if scan_count % 500 == 0 {
                    if let Ok(mut s) = scan_suffix.lock() {
                        *s = format!("\x1b[38;2;85;85;85m -> {}\x1b[0m", path.display());
                    }
                }
            }
        }
        scan_done.store(true, Ordering::Relaxed);
        scan_handle.join().unwrap();
        target_files = files;
        pb.finish_and_clear();
        println!("{} Directory Scan (Respecting .gitignore)", "[+]".green());
    }

    if target_files.is_empty() {
        println!("{}", "[!] No files found to pack.".yellow());
        return;
    }

    let (processable, skipped): (Vec<_>, Vec<_>) = target_files
        .into_iter()
        .partition(|p| !should_skip_file(p, args.no_limit, &ignore_globset));

    if args.dry_run {
        println!("{} {} files selected:", "[+]".green(), processable.len());
        for path in &processable {
            println!("  {}", path.display());
        }
        println!(
            "{} {} files skipped by safety limits.",
            "[+]".green(),
            skipped.len()
        );
        return;
    }

    if processable.is_empty() {
        println!(
            "{}",
            "[!] All files were skipped by safety limits.".yellow()
        );
        return;
    }

    let pb = ProgressBar::new(processable.len() as u64);
    pb.set_style(progress_style());

    let (pack_suffix, pack_done, pack_handle) =
        spawn_shimmer(pb.clone(), "Packing".to_string(), String::new());

    let mut cumulative_size = final_output.len();
    let mut file_count = 0;
    let mut limit_reached = false;

    for path in processable {
        if file_count % 50 == 0 {
            if let Ok(mut s) = pack_suffix.lock() {
                *s = format!("\x1b[38;2;85;85;85m {}\x1b[0m", path.display());
            }
        }

        if !running.load(Ordering::SeqCst) {
            println!("{}", "[!] Aborted by user.".yellow());
            break;
        }

        if is_binary_file(&path) {
            pb.inc(1);
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let lang = get_language(&ext);
            let mut processed = redact_secrets(&content);
            if minify {
                if let Some(lang) = lang {
                    processed = strip_comments(&processed, lang);
                }
                processed = minify_code(&processed);
            }

            if cumulative_size + processed.len() > max_output_bytes {
                println!(
                    "{}",
                    format!(
                        "[!] Reached {}MB output limit. Stopping early.",
                        max_output_bytes / (1024 * 1024)
                    )
                    .yellow()
                );
                limit_reached = true;
                break;
            }
            cumulative_size += processed.len();

            final_output.push_str(&format!(
                "### File: `{}`\n```{}\n{}\n```\n\n",
                path.display(),
                ext,
                processed
            ));
            file_count += 1;
        }

        pb.inc(1);
    }

    pack_done.store(true, Ordering::Relaxed);
    pack_handle.join().unwrap();

    if limit_reached {
        final_output.push_str("\n*Output truncated: reached configured output size limit.*\n");
    }

    pb.finish_and_clear();

    let token_estimate = count_tokens(&final_output);

    println!(
        "{} Packed {} files ({} skipped by limits).",
        "[+]".green(),
        file_count.to_string().cyan(),
        skipped.len().to_string().cyan()
    );
    println!(
        "{} Estimated Size: ~{} tokens",
        "[+]".green(),
        token_estimate.to_string().cyan()
    );
    if token_estimate > 100_000 {
        println!(
            "{}",
            "[!] Warning: Large payload. May exceed standard LLM context windows.".yellow()
        );
    }

    if let Some(output_path) = args.output {
        if let Err(e) = fs::write(&output_path, &final_output) {
            println!(
                "{} Failed to write output to {}: {}",
                "[!]".red(),
                output_path.display().to_string().cyan(),
                e
            );
        } else {
            println!(
                "{} SUCCESS: Context written to {}",
                "[+]".green(),
                output_path.display().to_string().cyan()
            );
        }
    } else {
        match Clipboard::new() {
            Ok(mut ctx) => {
                if let Err(e) = ctx.set_text(final_output.clone()) {
                    println!(
                        "{} Failed to write to clipboard: {}. Writing fallback file...",
                        "[!]".yellow(),
                        e
                    );
                    if let Err(e2) = fs::write("cxt-output.md", &final_output) {
                        println!("{} Also failed to write cxt-output.md: {}", "[!]".red(), e2);
                    } else {
                        println!(
                            "{} SUCCESS: Context written to cxt-output.md",
                            "[+]".green()
                        );
                    }
                } else {
                    println!(
                        "{} SUCCESS: Context safely copied to clipboard!",
                        "[+]".green()
                    );
                }
            }
            Err(e) => {
                println!(
                    "{} Clipboard error: {}. Writing context to cxt-output.md instead...",
                    "[!]".yellow(),
                    e
                );
                if let Err(e2) = fs::write("cxt-output.md", &final_output) {
                    println!("{} Failed to write cxt-output.md: {}", "[!]".red(), e2);
                } else {
                    println!(
                        "{} SUCCESS: Context written to cxt-output.md",
                        "[+]".green()
                    );
                }
            }
        }
    }
}
