use crate::cli::{ApplyArgs, Cli};
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::core::synchronization::{plan_changes, sync_references};
use crate::core::verification;
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::commit;
use crate::output::{human, json};

pub fn run(cli: &Cli, args: &ApplyArgs) -> Result<i32> {
    let target = Version::parse(&args.version)?;

    let repo = crate::cli::repo(cli);
    let config = ProjectConfig::load(&repo.root)?;
    let detection = detection::detect(&repo, &config)?;

    let Some(current) = detection.canonical.clone() else {
        return Err(VdriftError::Unsupported(
            "No canonical version source detected in this project.".into(),
        ));
    };

    let changes = plan_changes(&detection, &target);
    if changes.is_empty() {
        if cli.json {
            let out = json::ApplyOutput {
                status: "updated".into(),
                version: json::VersionOut {
                    from: current.to_string(),
                    to: target.to_string(),
                },
                files_changed: Vec::new(),
            };
            let rendered = serde_json::to_string_pretty(&out)
                .map_err(|e| VdriftError::Internal(e.to_string()))?;
            println!("{rendered}");
        } else {
            println!("✓ nothing to update");
        }
        return Ok(0);
    }

    if cli.dry_run {
        if cli.json {
            let out = json::ApplyOutput {
                status: "dry_run".into(),
                version: json::VersionOut {
                    from: current.to_string(),
                    to: target.to_string(),
                },
                files_changed: changes
                    .iter()
                    .map(|c| {
                        c.file
                            .strip_prefix(&repo.root)
                            .unwrap_or(&c.file)
                            .display()
                            .to_string()
                    })
                    .collect(),
            };
            let rendered = serde_json::to_string_pretty(&out)
                .map_err(|e| VdriftError::Internal(e.to_string()))?;
            println!("{rendered}");
        } else {
            println!("Files to update:");
            println!();
            for c in &changes {
                human::change_row(c, &repo.root);
            }
            println!();
            println!("[dry-run] no files were modified.");
        }
        return Ok(0);
    }

    let applied = sync_references(&repo, &detection.refs, &target, &config, false, args.force)?;

    let should_commit = args.commit && !args.no_commit;
    if should_commit {
        if !repo.is_git_repo() {
            return Err(VdriftError::Git(
                "cannot commit: not inside a Git repository".into(),
            ));
        }
        let files: Vec<std::path::PathBuf> = applied.iter().map(|c| c.file.clone()).collect();
        let msg = commit::bump_message(&target);
        commit::create_commit(&repo, &files, &msg)?;
    }

    let ver = verification::verify(&detection.refs, Some(target.clone()));

    if cli.json {
        let out = json::ApplyOutput {
            status: "updated".into(),
            version: json::VersionOut {
                from: current.to_string(),
                to: target.to_string(),
            },
            files_changed: applied
                .iter()
                .map(|c| {
                    c.file
                        .strip_prefix(&repo.root)
                        .unwrap_or(&c.file)
                        .display()
                        .to_string()
                })
                .collect(),
        };
        let rendered =
            serde_json::to_string_pretty(&out).map_err(|e| VdriftError::Internal(e.to_string()))?;
        println!("{rendered}");
    } else {
        for c in &applied {
            let file = c
                .file
                .strip_prefix(&repo.root)
                .unwrap_or(&c.file)
                .display()
                .to_string();
            println!("✓ {file}");
        }
        if ver.is_consistent() {
            println!();
            println!("✓ version consistency verified");
        }
    }

    Ok(0)
}
