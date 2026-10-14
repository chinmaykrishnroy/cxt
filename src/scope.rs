use std::path::{Path, PathBuf};

pub fn check(root: Option<&Path>, path: &Path) -> Result<PathBuf, String> {
    let Some(root) = root else {
        return Ok(path.to_path_buf());
    };
    let root = root
        .canonicalize()
        .map_err(|_| "workspace root is unavailable")?;
    let target = path
        .canonicalize()
        .map_err(|_| "workspace input is unavailable")?;
    if !root.is_dir() || !target.starts_with(&root) {
        return Err("path is outside the authorized workspace".into());
    }
    Ok(target)
}
