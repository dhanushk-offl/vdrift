use crate::git::repository::{Repository, git};

/// The last commit that touched `file` (used as the version-change baseline).
pub fn last_commit_for(repo: &Repository, file: &std::path::Path) -> Option<String> {
    let rel = file
        .strip_prefix(&repo.root)
        .unwrap_or(file)
        .to_string_lossy()
        .to_string();
    git(&repo.root, &["log", "-1", "--format=%H", "--", &rel])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Commit subjects since the given baseline commit (inclusive-exclusive).
/// With no baseline, falls back to the most recent 50 commits.
pub fn messages_since(repo: &Repository, baseline: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    match baseline {
        Some(from) => args.extend(["log".into(), "--format=%s".into(), format!("{from}..HEAD")]),
        None => args.extend([
            "log".into(),
            "--format=%s".into(),
            "-n".into(),
            "50".into(),
            "HEAD".into(),
        ]),
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match git(&repo.root, &args) {
        Ok(out) => out
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Parse pre-push stdin (`<local ref> <local oid> <remote ref> <remote oid>`)
/// and collect the subjects of commits being pushed.
pub fn push_messages(repo: &Repository, stdin: &str) -> Vec<String> {
    let mut messages = Vec::new();
    for line in stdin.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let local_oid = parts[1];
        let remote_oid = parts[3];
        if local_oid.chars().all(|c| c == '0') {
            continue; // branch deletion
        }
        if remote_oid.chars().all(|c| c == '0') {
            // New branch: fall back to commits since the last version change.
            let base = super::commit::last_bump_commit(repo);
            messages.extend(messages_since(repo, base.as_deref()));
            continue;
        }
        let args = ["log", "--format=%s", &format!("{remote_oid}..{local_oid}")];
        if let Ok(out) = git(&repo.root, &args) {
            messages.extend(
                out.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty()),
            );
        }
    }
    messages
}

/// Whether the environment is non-interactive (CI, agents, detached stdin).
///
/// Git hooks receive their input on a pipe even when run from a terminal, so
/// interactivity is judged from stderr (the controlling terminal) rather than
/// stdin alone.
pub fn is_non_interactive() -> bool {
    if std::env::var_os("CI").is_some()
        || std::env::var_os("VDRIFT_SKIP").is_some()
        || std::env::var_os("VDRIFT_RUNNING").is_some()
    {
        return true;
    }
    use std::io::IsTerminal;
    !std::io::stdin().is_terminal() && !std::io::stderr().is_terminal()
}
