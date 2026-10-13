use arboard::Clipboard;
use clap::Parser;
use cxt_core::cli::Args;
use cxt_core::engine::{self, EngineError, EngineResult, SelectionPreview};
use cxt_core::output::Compression;
use cxt_core::tui;
use flate2::write::GzEncoder;
use flate2::Compression as GzipLevel;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{self, Cursor, IsTerminal, Write};
use std::path::Path;
use std::time::Duration;

fn main() {
    let parsed = Args::parse();
    if let Some(shell) = parsed.completions {
        Args::write_completions(shell);
        return;
    }
    let mut args = parsed.normalized();
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("[cxt] failed to read current directory: {}", e);
            std::process::exit(2);
        }
    };

    let mut reviewed_result: Option<EngineResult> = None;
    if args.tui {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            eprintln!("[cxt] --tui requires an interactive terminal on stdin/stdout. Use CLI flags with --stdout, --ci or --output for scripts.");
            std::process::exit(2);
        }
        match tui::run(&args, &cwd) {
            Ok(Some(selection)) => {
                args = selection.args;
                reviewed_result = Some(selection.result);
            }
            Ok(None) => return,
            Err(message) => {
                eprintln!("[cxt] TUI error: {}", message);
                std::process::exit(2);
            }
        }
    }

    let result = match reviewed_result {
        Some(result) => Ok(result),
        None => match prepare_with_progress(&args, &cwd) {
            Ok(selection) => pack_with_progress(&args, &cwd, selection),
            Err(error) => Err(error),
        },
    };

    match result {
        Ok(result) => {
            if args.dry_run {
                print_dry_run(&result);
                return;
            }
            match deliver(&args, &result) {
                Ok(destination) => print_summary(&args, &result, &destination),
                Err(e) => {
                    eprintln!("[cxt] {}", e);
                    std::process::exit(3);
                }
            }
        }
        Err(EngineError::Secret(message)) => {
            eprintln!("[cxt] {}", message);
            std::process::exit(4);
        }
        Err(EngineError::General(message)) => {
            eprintln!("[cxt] {}", message);
            std::process::exit(2);
        }
    }
}

fn progress_enabled(args: &Args) -> bool {
    io::stderr().is_terminal() && !args.ci
}

fn prepare_with_progress(args: &Args, cwd: &Path) -> Result<SelectionPreview, EngineError> {
    if !progress_enabled(args) {
        return engine::prepare(args, cwd);
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message("Scanning project, applying filters and building context candidates…");
    let result = engine::prepare(args, cwd);
    pb.finish_and_clear();
    result
}

fn pack_with_progress(
    args: &Args,
    cwd: &Path,
    prepared: SelectionPreview,
) -> Result<EngineResult, EngineError> {
    if !progress_enabled(args) {
        return engine::run_prepared(args, cwd, prepared);
    }

    let total = prepared.candidates.len();
    let root = prepared.root.clone();
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} [{bar:28.cyan/blue}] {pos:>4}/{len:4} {msg}")
            .unwrap()
            .progress_chars("━━╸"),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message("Packing context…");

    let result =
        engine::run_prepared_with_progress(args, cwd, prepared, |index, count, path, bytes| {
            pb.set_length(count as u64);
            pb.set_position(index.saturating_sub(1) as u64);
            let display = path.strip_prefix(&root).unwrap_or(path);
            let large = if bytes >= 256 * 1024 {
                " • large"
            } else {
                ""
            };
            pb.set_message(format!(
                "{} • {}{}",
                compact_path(display, 62),
                human_bytes(bytes),
                large
            ));
        });
    pb.set_position(total as u64);
    pb.finish_and_clear();
    result
}

fn compact_path(path: &Path, max_chars: usize) -> String {
    let text = path.display().to_string();
    let count = text.chars().count();
    if count <= max_chars {
        return text;
    }
    let tail: String = text
        .chars()
        .rev()
        .take(max_chars.saturating_sub(1))
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("…{}", tail)
}

fn human_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

enum Delivery {
    File(std::path::PathBuf),
    Stdout,
    Clipboard,
    FallbackFile(std::path::PathBuf),
}

fn deliver(args: &Args, result: &EngineResult) -> Result<Delivery, String> {
    if let Some(path) = &args.output {
        write_file(path, args.compression, result.rendered.as_bytes())?;
        return Ok(Delivery::File(path.clone()));
    }

    if args.stdout || args.ci {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(result.rendered.as_bytes())
            .map_err(|e| e.to_string())?;
        if !result.rendered.ends_with('\n') {
            stdout.write_all(b"\n").map_err(|e| e.to_string())?;
        }
        return Ok(Delivery::Stdout);
    }

    match Clipboard::new().and_then(|mut c| c.set_text(result.rendered.clone())) {
        Ok(_) => Ok(Delivery::Clipboard),
        Err(error) => {
            let fallback = match result.format {
                cxt_core::output::OutputFormat::Markdown => "cxt-output.md",
                cxt_core::output::OutputFormat::Xml => "cxt-output.xml",
                cxt_core::output::OutputFormat::Json => "cxt-output.json",
            };
            let mut file = fs::OpenOptions::new().write(true).create_new(true).open(fallback).map_err(|write_error| {
                format!(
                    "clipboard failed ({}), and fallback {} could not be created: {}. Use --stdout or --output with a new filename.",
                    error, fallback, write_error
                )
            })?;
            file.write_all(result.rendered.as_bytes())
                .map_err(|e| e.to_string())?;
            eprintln!(
                "[cxt] clipboard unavailable; wrote context to {} instead",
                fallback
            );
            Ok(Delivery::FallbackFile(fallback.into()))
        }
    }
}

fn write_file(path: &Path, compression: Compression, bytes: &[u8]) -> Result<(), String> {
    match compression {
        Compression::None => fs::write(path, bytes).map_err(|e| e.to_string()),
        Compression::Gzip => {
            let file = fs::File::create(path).map_err(|e| e.to_string())?;
            let mut encoder = GzEncoder::new(file, GzipLevel::default());
            encoder.write_all(bytes).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())?;
            Ok(())
        }
        Compression::Zstd => {
            let compressed =
                zstd::stream::encode_all(Cursor::new(bytes), 3).map_err(|e| e.to_string())?;
            fs::write(path, compressed).map_err(|e| e.to_string())
        }
    }
}

fn print_dry_run(result: &EngineResult) {
    println!(
        "[cxt] {} files selected (mode: {}, root: {})",
        result.packed_files.len(),
        result.selection_mode,
        result.root.display()
    );
    for file in &result.packed_files {
        let path = file.path.strip_prefix(&result.root).unwrap_or(&file.path);
        println!(
            "  {}  ~{} tokens  score={}  [{}]",
            path.display(),
            file.tokens,
            file.score,
            file.reasons.join(", ")
        );
    }
    if result.budget_skipped > 0 {
        println!(
            "[cxt] {} additional files skipped by token budget",
            result.budget_skipped
        );
    }
    print_skipped_details(result);
}

fn print_skipped_details(result: &EngineResult) {
    if result.skipped_details.is_empty() {
        return;
    }
    eprintln!(
        "[cxt] {} unreadable/binary/walk-error item(s) were skipped:",
        result.skipped_details.len()
    );
    for item in result.skipped_details.iter().take(8) {
        match &item.path {
            Some(path) => {
                let display = path.strip_prefix(&result.root).unwrap_or(path);
                eprintln!("  - {}: {}", display.display(), item.reason);
            }
            None => eprintln!("  - {}", item.reason),
        }
    }
    if result.skipped_details.len() > 8 {
        eprintln!(
            "  … and {} more (open the TUI and press 4 to inspect skipped items)",
            result.skipped_details.len() - 8
        );
    }
}

fn print_summary(args: &Args, result: &EngineResult, destination: &Delivery) {
    let summary = format!(
        "[cxt] packed {} files, ~{} tokens, {} redactions, {} skipped ({} by budget)",
        result.packed_files.len(),
        result.estimated_tokens,
        result.redactions,
        result.skipped_files,
        result.budget_skipped
    );

    if args.stdout || (args.ci && args.output.is_none()) {
        eprintln!("{}", summary);
    } else {
        println!("{}", summary);
        match destination {
            Delivery::File(path) => println!("[cxt] wrote {}", path.display()),
            Delivery::FallbackFile(path) => {
                println!("[cxt] wrote {} (clipboard unavailable)", path.display())
            }
            Delivery::Clipboard => println!("[cxt] copied context to clipboard"),
            Delivery::Stdout => {}
        }
    }
    print_skipped_details(result);
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use cxt_core::cli::Args;

    #[test]
    fn bare_invocation_defaults_to_cli_and_current_directory() {
        let args = Args::parse_from(["cxt"]).normalized();
        assert!(!args.tui);
        assert_eq!(args.paths, vec![std::path::PathBuf::from(".")]);
    }

    #[test]
    fn tui_is_explicit_and_does_not_steal_trace_short_flag() {
        assert!(Args::parse_from(["cxt", "--tui"]).tui);
        assert!(Args::parse_from(["cxt", "-T"]).tui);
        let args = Args::parse_from(["cxt", "-t", "main.rs"]);
        assert!(args.trace);
        assert!(!args.tui);
    }
}
