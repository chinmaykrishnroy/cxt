use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the path to the `cxt` binary under test.
///
/// Cargo sets `CARGO_BIN_EXE_cxt` for integration tests; we fall back to a
/// conventional debug path when running outside of `cargo test`.
pub fn cxt_bin() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_cxt")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = PathBuf::from("target/debug");
            if cfg!(windows) {
                path.push("cxt.exe");
            } else {
                path.push("cxt");
            }
            path
        })
}

/// Creates a fresh, unique temporary directory for a test case.
pub fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("cxt-test-{}-{}-{}", prefix, pid, nanos));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `content` to `dir/name`, creating any intermediate directories.
pub fn write_file(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Writes raw bytes to `dir/name`.
pub fn write_bytes(dir: &Path, name: &str, content: &[u8]) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Runs `cxt` with the supplied arguments from `cwd`.
pub fn run_cxt(cwd: &Path, args: &[&str]) -> Output {
    Command::new(cxt_bin())
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .args(args)
        .output()
        .expect("failed to execute cxt")
}

/// Asserts that `cxt` exited successfully and prints a useful message on failure.
pub fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "cxt exited with non-zero status.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Returns the contents of `dir/name`.
pub fn read_file(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).unwrap()
}

/// Initializes a git repository in `dir` so that `.gitignore` files are
/// respected by `cxt`'s directory walker.
pub fn init_git_repo(dir: &Path) {
    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(dir)
        .status()
        .expect("git is required for this test");
    assert!(status.success(), "git init failed");
}

/// Writes `content` to `dir/name` and commits it in the local git repository.
/// Used to set up a baseline for `--diff` tests.
pub fn commit_file(dir: &Path, name: &str, content: &str, message: &str) {
    write_file(dir, name, content);
    let status = Command::new("git")
        .arg("add")
        .arg(name)
        .current_dir(dir)
        .status()
        .expect("git add failed");
    assert!(status.success(), "git add failed");

    let status = Command::new("git")
        .arg("-c")
        .arg("user.name=cxt-test")
        .arg("-c")
        .arg("user.email=cxt-test@example.com")
        .arg("commit")
        .arg("-q")
        .arg("-m")
        .arg(message)
        .current_dir(dir)
        .status()
        .expect("git commit failed");
    assert!(status.success(), "git commit failed");
}
