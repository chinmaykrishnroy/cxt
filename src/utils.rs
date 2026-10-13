use std::fs;
use std::io::Read;
use std::path::Path;

const BINARY_SAMPLE_SIZE: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileProbe {
    Text,
    Binary,
    Unreadable(String),
}

/// Cheaply checks whether a candidate can actually be opened/read before it is
/// shown in the TUI or handed to the packing pipeline.
///
/// This deliberately reads only a small prefix. Full UTF-8 validation still
/// happens in `fs::read_to_string` during packing, where any late error is
/// recorded and skipped rather than aborting the run.
pub fn probe_file(path: &Path) -> FileProbe {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => return FileProbe::Unreadable(error.to_string()),
    };

    let mut buf = [0u8; BINARY_SAMPLE_SIZE];
    match file.read(&mut buf) {
        Ok(n) if n > 0 && buf[..n].contains(&0x00) => FileProbe::Binary,
        Ok(_) => FileProbe::Text,
        Err(error) => FileProbe::Unreadable(error.to_string()),
    }
}

/// Returns true if the file at `path` appears to be binary.
///
/// Unreadable files are *not* treated as text; callers that care about the
/// distinction should use `probe_file` so they can report the read error.
pub fn is_binary_file(path: &Path) -> bool {
    matches!(probe_file(path), FileProbe::Binary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cxt-probe-{}-{}-{}",
            std::process::id(),
            stamp,
            name
        ))
    }

    #[test]
    fn probe_distinguishes_text_binary_and_missing() {
        let text = temp_path("text");
        let binary = temp_path("binary");
        fs::write(&text, b"hello\n").unwrap();
        fs::write(&binary, b"a\0b").unwrap();

        assert_eq!(probe_file(&text), FileProbe::Text);
        assert_eq!(probe_file(&binary), FileProbe::Binary);
        assert!(matches!(
            probe_file(&temp_path("missing")),
            FileProbe::Unreadable(_)
        ));

        let _ = fs::remove_file(text);
        let _ = fs::remove_file(binary);
    }
}
