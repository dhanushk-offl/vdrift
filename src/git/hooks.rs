use crate::errors::{Result, VdriftError};
use std::path::Path;
use std::process::Command;

pub const PRE_PUSH_HOOK: &str = "pre-push";

/// The dispatcher shell script Git executes. All intelligence lives in the binary.
pub const HOOK_SCRIPT: &str = "#!/bin/sh\n\
# vdrift global pre-push dispatcher\n\
# version drift shouldn't happen.\n\
exec vdrift hook pre-push \"$@\"\n";

/// Writes the dispatcher hook into `dir` and marks it executable.
pub fn write_dispatcher(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)
        .map_err(|e| VdriftError::Config(format!("cannot create {}: {e}", dir.display())))?;
    let hook = dir.join(PRE_PUSH_HOOK);
    std::fs::write(&hook, HOOK_SCRIPT)
        .map_err(|e| VdriftError::Config(format!("cannot write {}: {e}", hook.display())))?;
    make_executable(&hook)?;
    Ok(())
}

/// Removes the dispatcher hook (and the hooks directory if it becomes empty).
pub fn remove_dispatcher(dir: &Path) -> Result<()> {
    let hook = dir.join(PRE_PUSH_HOOK);
    if hook.exists() {
        std::fs::remove_file(&hook)
            .map_err(|e| VdriftError::Config(format!("cannot remove {}: {e}", hook.display())))?;
    }
    Ok(())
}

/// Current value of `git config --global core.hooksPath`.
pub fn current_hooks_path() -> Result<Option<String>> {
    let out = Command::new("git")
        .args(["config", "--global", "--get", "core.hooksPath"])
        .output()
        .map_err(|e| VdriftError::Git(format!("failed to run git config: {e}")))?;
    if !out.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

/// Sets `git config --global core.hooksPath`.
pub fn set_hooks_path(path: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["config", "--global", "core.hooksPath", path])
        .status()
        .map_err(|e| VdriftError::Git(format!("failed to set core.hooksPath: {e}")))?;
    if !status.success() {
        return Err(VdriftError::Git(
            "git config --global core.hooksPath failed".into(),
        ));
    }
    Ok(())
}

/// Unsets `git config --global core.hooksPath`.
pub fn unset_hooks_path() -> Result<()> {
    let status = Command::new("git")
        .args(["config", "--global", "--unset", "core.hooksPath"])
        .status()
        .map_err(|e| VdriftError::Git(format!("failed to unset core.hooksPath: {e}")))?;
    if !status.success() {
        // Ignore "no such key" errors.
        let _ = status;
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| VdriftError::Config(format!("cannot stat {}: {e}", path.display())))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)
            .map_err(|e| VdriftError::Config(format!("cannot chmod {}: {e}", path.display())))?;
    }
    Ok(())
}
