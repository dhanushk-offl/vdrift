use crate::config::global::GlobalConfig;
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::core::proposal;
use crate::errors::Result;
use crate::git::diff;
use crate::git::repository::Repository;
use std::io::Read;

/// Global pre-push dispatcher entry point (invoked by the Git hook).
///
/// Guards run first so vdrift never becomes annoying or recursive, then the
/// interactive version flow decides whether the push should proceed.
pub fn run_pre_push() -> Result<i32> {
    // 1. Recursion protection — vdrift's own Git operations must not re-trigger.
    if std::env::var_os("VDRIFT_RUNNING").is_some() {
        return Ok(0);
    }
    // 2. Explicit one-time bypass.
    if std::env::var_os("VDRIFT_SKIP").is_some() {
        return Ok(0);
    }
    // 3. Global integration disabled.
    let global = GlobalConfig::load()?;
    if !global.enabled {
        return Ok(0);
    }
    // 4. Not inside a Git repository.
    let repo = Repository::discover(&std::env::current_dir().unwrap_or_default());
    if !repo.is_git_repo() {
        return Ok(0);
    }
    // 5. Repository opted out.
    let config = ProjectConfig::load(&repo.root)?;
    if !config.enabled() {
        return Ok(0);
    }
    // 6. Non-interactive environments (CI, pipes, agents) bypass automatically.
    if diff::is_non_interactive() {
        return Ok(0);
    }

    // 7. What is actually being pushed?
    let mut stdin = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin)
        .map_err(|e| crate::errors::VdriftError::Internal(e.to_string()))?;
    let messages = diff::push_messages(&repo, &stdin);

    // 8. Only perform deep analysis when there is something to look at.
    let detection = detection::detect(&repo, &config)?;
    let Some(_canonical) = detection.canonical.clone() else {
        return Ok(0);
    };
    let proposal = proposal::analyze(&messages);
    let drift = detection.drifted_refs();

    if !proposal.needs_update() && drift.is_empty() {
        println!("Everything looks good.");
        return Ok(0);
    }

    // 9. Interactive flow. Stops the push so the version commit can be included.
    crate::cli::root::run_flow(crate::cli::root::Flow {
        repo: &repo,
        config: &config,
        detection: &detection,
        messages,
        stop_push: true,
        json: false,
        dry_run: false,
    })
}
