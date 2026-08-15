use crate::errors::{Result, VdriftError};
use crate::git::repository::{Repository, git};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Finds the most recent commit that bumped the version, by message pattern.
pub fn last_bump_commit(repo: &Repository) -> Option<String> {
    git(
        &repo.root,
        &[
            "log",
            "--format=%H",
            "--grep=bump version",
            "--grep=release:",
            "--grep=release(",
            "--max-count=1",
        ],
    )
    .ok()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
}

fn relative_paths(root: &Path, files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            f.strip_prefix(root)
                .unwrap_or(f)
                .to_string_lossy()
                .to_string()
        })
        .collect()
}

/// Stages the given files (only the files vdrift just modified).
pub fn stage_files(repo: &Repository, files: &[PathBuf]) -> Result<()> {
    let rel = relative_paths(&repo.root, files);
    if rel.is_empty() {
        return Ok(());
    }
    let mut args = vec!["add", "--"];
    args.extend(rel.iter().map(String::as_str));
    git(&repo.root, &args)?;
    Ok(())
}

/// Creates a commit containing exactly the given files.
///
/// Sets `VDRIFT_RUNNING=1` so any hooks triggered by this operation (e.g. a
/// pre-commit hook) skip their own vdrift cycle.
pub fn create_commit(repo: &Repository, files: &[PathBuf], message: &str) -> Result<()> {
    stage_files(repo, files)?;

    let output = Command::new("git")
        .arg("-C")
        .arg(&repo.root)
        .args(["commit", "-m", message])
        .env("VDRIFT_RUNNING", "1")
        .env("VDRIFT_SKIP", "1")
        .output()
        .map_err(|e| VdriftError::Git(format!("failed to run git commit: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(VdriftError::Git(if stderr.is_empty() {
            "git commit failed".into()
        } else {
            stderr
        }));
    }
    Ok(())
}

/// Default commit message for a version bump.
pub fn bump_message(version: &crate::core::version::Version) -> String {
    format!("chore: bump version to {version}")
}
