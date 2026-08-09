use std::fs;
use std::io::Read;
use std::path::Path;

const BINARY_SAMPLE_SIZE: usize = 1024;

/// Returns true if the file at `path` appears to be binary.
///
/// This is a fast heuristic: read up to the first 1 KB and look for a NULL byte.
/// Empty files are treated as non-binary so they can fall through to the normal reader.
pub fn is_binary_file(path: &Path) -> bool {
    match fs::File::open(path) {
        Ok(mut file) => {
            let mut buf = [0u8; BINARY_SAMPLE_SIZE];
            match file.read(&mut buf) {
                Ok(n) if n > 0 => buf[..n].contains(&0x00),
                _ => false,
            }
        }
        Err(_) => false,
    }
}
