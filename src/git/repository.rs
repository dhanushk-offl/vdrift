use crate::errors::{Result, VdriftError};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A repository vdrift is operating on.
///
/// `root` is the directory vdrift was invoked from (or the explicit target).
/// `git_root` is the top-level of the enclosing Git repository, when one exists.
#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
    pub git_root: Option<PathBuf>,
}

impl Repository {
    /// Discovers the repository from a working directory. Always succeeds for
    /// the filesystem root; git detection is best-effort.
    pub fn discover(cwd: &Path) -> Repository {
        let root = if cwd.is_absolute() {
            cwd.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|d| d.join(cwd))
                .unwrap_or_else(|_| cwd.to_path_buf())
        };
        let root = root.canonicalize().unwrap_or_else(|_| root.clone());

        let git_root = git(&root, &["rev-parse", "--show-toplevel"])
            .ok()
            .map(|out| PathBuf::from(out.trim()))
            .filter(|p| !p.as_os_str().is_empty());

        Repository { root, git_root }
    }

    pub fn is_git_repo(&self) -> bool {
        self.git_root.is_some()
    }
}

/// Runs `git -C dir <args>` and returns trimmed stdout on success.
pub fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| VdriftError::Git(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(VdriftError::Git(if stderr.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            stderr
        }));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Returns true when the file carries uncommitted changes in the Git index or
/// working tree. Untracked files are not considered dirty (nothing can be lost).
pub fn is_dirty(repo: &Repository, file: &Path) -> Result<bool> {
    if !repo.is_git_repo() {
        return Ok(false);
    }
    let rel = file
        .strip_prefix(&repo.root)
        .unwrap_or(file)
        .to_string_lossy()
        .to_string();

    let out = git(&repo.root, &["status", "--porcelain", "--", &rel])?;
    for line in out.lines() {
        if line.len() < 3 {
            continue;
        }
        let bytes = line.as_bytes();
        // "??" = untracked, not dirty for our purposes.
        if bytes[0] == b'?' && bytes[1] == b'?' {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_plain_dir() {
        let repo = Repository::discover(&std::env::temp_dir());
        assert!(repo.root.is_absolute());
    }
}
