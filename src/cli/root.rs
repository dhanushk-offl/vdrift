use crate::cli::Cli;
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::core::proposal;
use crate::core::synchronization::{plan_changes, sync_references};
use crate::core::verification;
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::commit;
use crate::git::diff;
use crate::git::repository::Repository;
use crate::output::human;
use std::path::PathBuf;

/// Context for the shared interactive version flow, used by both the root
/// command and the global pre-push dispatcher.
pub struct Flow<'a> {
    pub repo: &'a Repository,
    pub config: &'a ProjectConfig,
    pub detection: &'a detection::DetectionResult,
    pub messages: Vec<String>,
    /// When true (pre-push), the flow stops the push so the version commit is
    /// included on the next `git push`.
    pub stop_push: bool,
    pub json: bool,
    pub dry_run: bool,
}

pub fn run(cli: &Cli) -> Result<i32> {
    let repo = crate::cli::repo(cli);
    let config = ProjectConfig::load(&repo.root)?;
    let detection = detection::detect(&repo, &config)?;

    let messages = if repo.is_git_repo() {
        let baseline = detection
            .canonical_file
            .as_ref()
            .and_then(|f| diff::last_commit_for(&repo, f));
        diff::messages_since(&repo, baseline.as_deref())
    } else {
        Vec::new()
    };

    run_flow(Flow {
        repo: &repo,
        config: &config,
        detection: &detection,
        messages,
        stop_push: false,
        json: cli.json,
        dry_run: cli.dry_run,
    })
}

pub fn run_flow(f: Flow) -> Result<i32> {
    let Some(canonical) = f.detection.canonical.clone() else {
        println!("No version detected in this project.");
        return Ok(0);
    };

    let proposal = proposal::analyze(&f.messages);
    let drifted = f.detection.drifted_refs();
    let bump_needed = proposal.needs_update();
    let target = canonical.bump(proposal.level);

    if !bump_needed && drifted.is_empty() {
        println!("Everything looks good.");
        return Ok(0);
    }

    if bump_needed {
        println!("version update detected");
    } else {
        println!("version drift detected");
    }
    println!();
    human::item("current", &canonical.to_string());
    human::item("suggested", &target.to_string());
    println!();

    if !f.config.auto_bump() && !f.dry_run && !f.json && !confirm("Update project version? [Y/n]")?
    {
        return Ok(0);
    }

    let mut chosen = target.clone();
    if !f.dry_run && !f.json {
        let term = dialoguer::console::Term::stderr();
        let input: String = dialoguer::Input::new()
            .with_prompt("New version")
            .default(target.to_string())
            .interact_text_on(&term)
            .map_err(|e| VdriftError::Cancelled(e.to_string()))?;
        chosen = Version::parse(&input)?;
    }

    let changes = plan_changes(f.detection, &chosen);
    if changes.is_empty() {
        println!("Nothing to update.");
        return Ok(0);
    }

    println!("Files to update:");
    println!();
    for c in &changes {
        human::change_row(c, &f.repo.root);
    }
    println!();

    if !f.config.auto_bump() && !f.dry_run && !f.json && !confirm("Continue? [Y/n]")? {
        return Ok(crate::errors::VdriftError::Cancelled("cancelled".into()).exit_code());
    }

    if f.dry_run {
        println!("[dry-run] no files were modified.");
        return Ok(0);
    }

    let applied = sync_references(f.repo, &f.detection.refs, &chosen, f.config, false, false)?;
    for c in &applied {
        let file = c
            .file
            .strip_prefix(&f.repo.root)
            .unwrap_or(&c.file)
            .display()
            .to_string();
        println!("✓ {file}");
    }

    let ver = verification::verify(&f.detection.refs, Some(chosen.clone()));
    if ver.is_consistent() {
        println!();
        println!("✓ version consistency verified");
    }

    if f.repo.is_git_repo() && !applied.is_empty() {
        println!();
        let default_msg = commit::bump_message(&chosen);
        if let Some(msg) = commit_action(&f, &default_msg)? {
            let files: Vec<PathBuf> = applied.iter().map(|c| c.file.clone()).collect();
            commit::create_commit(f.repo, &files, &msg)?;
            println!("✓ created commit");
            if f.stop_push {
                println!();
                println!("push was stopped so the new commit can be included.");
                println!();
                println!("Run:");
                println!("  git push");
                return Ok(1);
            }
            return Ok(0);
        }
        if f.stop_push {
            println!();
            println!("push was stopped — version files changed but were not committed.");
            return Ok(1);
        }
        println!("Skipped commit. Version files were updated in the working tree.");
    }

    Ok(0)
}

fn commit_action(f: &Flow, default_msg: &str) -> Result<Option<String>> {
    if f.json {
        return Ok(if f.config.auto_commit() {
            Some(default_msg.to_string())
        } else {
            None
        });
    }
    if f.config.auto_commit() {
        return Ok(Some(default_msg.to_string()));
    }

    let term = dialoguer::console::Term::stderr();
    let selection = dialoguer::Select::new()
        .with_prompt("Create version commit?")
        .items(&["create", "edit", "skip"])
        .default(0)
        .interact_on(&term)
        .map_err(|e| VdriftError::Cancelled(e.to_string()))?;

    match selection {
        0 => Ok(Some(default_msg.to_string())),
        1 => {
            let edited: String = dialoguer::Input::new()
                .with_prompt("Commit message")
                .default(default_msg.to_string())
                .interact_text_on(&term)
                .map_err(|e| VdriftError::Cancelled(e.to_string()))?;
            Ok(Some(edited))
        }
        _ => Ok(None),
    }
}

fn confirm(prompt: &str) -> Result<bool> {
    let term = dialoguer::console::Term::stderr();
    dialoguer::Confirm::new()
        .with_prompt(prompt)
        .default(true)
        .interact_on(&term)
        .map_err(|e| VdriftError::Cancelled(e.to_string()))
}
