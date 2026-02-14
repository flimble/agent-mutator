use std::path::{Path, PathBuf};

pub fn backup_path(source_file: &Path) -> PathBuf {
    let mut backup = source_file.to_path_buf();
    let name = format!(
        ".{}.mutator.bak",
        source_file.file_name().unwrap_or_default().to_string_lossy()
    );
    backup.set_file_name(name);
    backup
}

/// Check if a backup file exists from a previous interrupted in-place run.
pub fn check_interrupted_run(source_file: &Path) -> Option<PathBuf> {
    let bak = backup_path(source_file);
    if bak.exists() {
        Some(bak)
    } else {
        None
    }
}

/// Restore source from backup file (legacy in-place recovery).
pub fn restore_from_backup(source_file: &Path, backup_file: &Path) -> std::io::Result<()> {
    std::fs::copy(backup_file, source_file)?;
    std::fs::remove_file(backup_file)?;
    crate::runner::clear_pycache_for(source_file);
    Ok(())
}
