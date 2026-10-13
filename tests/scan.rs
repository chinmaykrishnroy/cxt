use clap::Parser;
use cxt_core::{cli::Args, engine, review::Workspace, scan::ScanLimits};
use std::{fs, path::PathBuf, process::Command, time::SystemTime};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "cxt-scan-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, contents: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    fn args(&self, flags: &[&str]) -> Args {
        Args::parse_from(std::iter::once("cxt").chain(flags.iter().copied())).normalized()
    }
    fn git(&self, flags: &[&str]) {
        assert!(Command::new("git")
            .args(flags)
            .current_dir(&self.0)
            .output()
            .unwrap()
            .status
            .success());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn home_and_filesystem_root_require_explicit_permission() {
    let home = directories::BaseDirs::new()
        .unwrap()
        .home_dir()
        .to_path_buf();
    let args = Args::parse_from(["cxt", "--dry-run"]).normalized();
    let error = engine::prepare(&args, &home).unwrap_err().to_string();
    assert!(error.contains("WARNING"));
    assert!(error.contains("--allow-broad-scan"));
    let root = home.ancestors().last().unwrap();
    assert!(ScanLimits::default().guard(root).is_err());
    assert!(ScanLimits {
        allow_broad: true,
        ..ScanLimits::default()
    }
    .guard(&home)
    .is_ok());
}

#[test]
fn entry_limit_stops_before_export_and_no_limit_does_not_bypass_it() {
    let f = Fixture::new();
    f.write("a.rs", "fn a() {}");
    f.write("b.rs", "fn b() {}");
    let output = Command::new(env!("CARGO_BIN_EXE_cxt"))
        .args([
            "--scan-max-entries",
            "1",
            "--no-limit",
            "--output",
            "out.md",
        ])
        .current_dir(&f.0)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("entry limit"));
    assert!(!f.0.join("out.md").exists());
    assert_eq!(
        engine::run(&f.args(&["--scan-max-entries", "10"]), &f.0)
            .unwrap()
            .packed_files
            .len(),
        2
    );
    assert!(engine::run(&f.args(&["--scan-max-entries", "0"]), &f.0).is_ok());
}

#[test]
fn depth_limit_is_an_error_not_a_silently_truncated_pack() {
    let f = Fixture::new();
    f.write("nested/deeper/a.rs", "fn a() {}");
    let error = engine::run(&f.args(&["--scan-max-depth", "1"]), &f.0).unwrap_err();
    assert!(error.to_string().contains("depth limit"));
    assert_eq!(
        engine::run(&f.args(&["--scan-max-depth", "3"]), &f.0)
            .unwrap()
            .packed_files
            .len(),
        1
    );
    assert!(engine::run(&f.args(&["--scan-max-depth", "0"]), &f.0).is_ok());
}

#[test]
fn byte_limit_applies_to_aggregate_input_even_with_per_file_limits_disabled() {
    let f = Fixture::new();
    f.write("a.txt", &"a".repeat(600 * 1024));
    f.write("b.txt", &"b".repeat(600 * 1024));
    let error = engine::prepare(&f.args(&["--no-limit", "--scan-max-mb", "1"]), &f.0).unwrap_err();
    assert!(error.to_string().contains("MiB input limit"));
    assert!(engine::prepare(&f.args(&["--scan-max-mb", "2"]), &f.0).is_ok());
    assert!(engine::prepare(&f.args(&["--scan-max-mb", "0"]), &f.0).is_ok());
}

#[test]
fn default_safety_exclusions_remain_counted_without_consuming_input_byte_budget() {
    let f = Fixture::new();
    f.write("a.rs", "fn a() {}");
    f.write("Cargo.lock", "generated lock data");
    f.write("large.txt", &"x".repeat(2 * 1024 * 1024));
    let prepared = engine::prepare(&f.args(&["--scan-max-mb", "1"]), &f.0).unwrap();
    assert_eq!(prepared.candidates.len(), 1);
    assert_eq!(prepared.skipped_files, 2);
}

#[test]
fn project_defaults_can_be_overridden_including_zero() {
    let f = Fixture::new();
    f.write("a.rs", "fn a() {}");
    f.write(
        ".cxt.toml",
        "scan_max_entries = 1\nscan_max_depth = 2\nscan_max_mb = 1\nscan_timeout = 30\n",
    );
    assert!(engine::prepare(&f.args(&[]), &f.0)
        .unwrap_err()
        .to_string()
        .contains("entry limit"));
    let args = f.args(&["--scan-max-entries", "0"]);
    let limits = ScanLimits::for_args(&args, &f.0);
    assert_eq!(limits.max_entries, 0);
    assert_eq!(limits.max_depth, 2);
    assert_eq!(limits.max_bytes, 1024 * 1024);
    assert_eq!(limits.timeout.as_secs(), 30);
    assert!(engine::prepare(&args, &f.0).is_ok());
}

#[test]
fn tracing_is_bounded_even_when_input_is_a_single_file() {
    let f = Fixture::new();
    f.write("app.ts", "import './helper';");
    f.write("helper.ts", "import './third';");
    f.write("third.ts", "export const answer = 42;");
    for flag in ["--trace", "--query"] {
        let flags = if flag == "--trace" {
            vec!["app.ts", flag, "--scan-max-entries", "2"]
        } else {
            vec!["app.ts", flag, "app", "--scan-max-entries", "2"]
        };
        assert!(engine::prepare(&f.args(&flags), &f.0)
            .unwrap_err()
            .to_string()
            .contains("entry limit"));
    }
    assert_eq!(
        engine::run(
            &f.args(&["app.ts", "--trace", "--scan-max-entries", "10"]),
            &f.0
        )
        .unwrap()
        .packed_files
        .len(),
        3
    );
}

#[test]
fn tui_graph_and_impact_cannot_bypass_entry_limits_with_a_small_seed() {
    let f = Fixture::new();
    f.write("app.ts", "export const app = 1;");
    f.write("helper.ts", "import './app';");
    f.write("third.ts", "export const answer = 42;");
    let args = f.args(&["app.ts", "--scan-max-entries", "2"]);
    assert!(engine::prepare(&args, &f.0).is_ok());
    assert!(Workspace::load(&args, &f.0)
        .unwrap_err()
        .contains("entry limit"));
    f.git(&["init", "-q"]);
    f.git(&["add", "app.ts"]);
    f.git(&[
        "-c",
        "user.name=cxt-test",
        "-c",
        "user.email=test@example.com",
        "commit",
        "-qm",
        "baseline",
    ]);
    f.write("app.ts", "export const app = 2;");
    let error = engine::prepare(
        &f.args(&["--git", "unstaged", "--impact", "--scan-max-entries", "2"]),
        &f.0,
    )
    .unwrap_err();
    assert!(error.to_string().contains("entry limit"));
}

#[test]
fn git_patch_review_including_deletions_is_bounded() {
    let f = Fixture::new();
    f.git(&["init", "-q"]);
    for name in ["a.rs", "b.rs", "c.rs"] {
        f.write(name, "fn a() {}");
    }
    f.git(&["add", "."]);
    f.git(&[
        "-c",
        "user.name=cxt-test",
        "-c",
        "user.email=test@example.com",
        "commit",
        "-qm",
        "baseline",
    ]);
    for name in ["a.rs", "b.rs", "c.rs"] {
        fs::remove_file(f.0.join(name)).unwrap();
    }
    f.write("anchor.rs", "fn anchor() {}");
    assert!(
        Workspace::load(&f.args(&["anchor.rs", "--scan-max-entries", "2"]), &f.0)
            .unwrap_err()
            .contains("entry limit")
    );
}

#[test]
fn older_saved_options_get_safe_defaults() {
    let mut json = serde_json::to_value(Args::parse_from(["cxt"])).unwrap();
    for name in [
        "allow_broad_scan",
        "scan_max_entries",
        "scan_max_mb",
        "scan_max_depth",
        "scan_timeout",
    ] {
        json.as_object_mut().unwrap().remove(name);
    }
    let args: Args = serde_json::from_value(json).unwrap();
    assert!(!args.allow_broad_scan);
    assert!(args.scan_max_entries.is_none());
    assert!(args.scan_max_mb.is_none());
    assert!(args.scan_max_depth.is_none());
    assert!(args.scan_timeout.is_none());
}

#[test]
fn explicit_tui_requires_a_terminal_but_bare_cli_with_ci_does_not() {
    let f = Fixture::new();
    f.write("a.rs", "fn a() {}");
    let output = Command::new(env!("CARGO_BIN_EXE_cxt"))
        .arg("-T")
        .current_dir(&f.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("terminal"));
    let output = Command::new(env!("CARGO_BIN_EXE_cxt"))
        .arg("--ci")
        .current_dir(&f.0)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("fn a() {}"));
}
