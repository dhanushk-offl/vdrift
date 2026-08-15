use crate::cli::{Cli, SyncArgs};
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::core::synchronization::{plan_changes, sync_references};
use crate::core::verification;
use crate::errors::{Result, VdriftError};
use crate::output::{human, json};

pub fn run(cli: &Cli, args: &SyncArgs) -> Result<i32> {
    let repo = crate::cli::repo(cli);
    let config = ProjectConfig::load(&repo.root)?;
    let detection = detection::detect(&repo, &config)?;

    let Some(canonical) = detection.canonical.clone() else {
        if cli.json {
            let rendered = serde_json::to_string_pretty(&json::error_body(
                "UNSUPPORTED_PROJECT",
                "No canonical version source detected in this project.",
            ))
            .map_err(|e| VdriftError::Internal(e.to_string()))?;
            println!("{rendered}");
        } else {
            println!("✗ no project version detected in this directory");
        }
        return Err(VdriftError::Unsupported(
            "No canonical version source detected in this project.".into(),
        ));
    };

    let changes = plan_changes(&detection, &canonical);
    if changes.is_empty() {
        if cli.json {
            let out = json::ApplyOutput {
                status: "synchronized".into(),
                version: json::VersionOut {
                    from: canonical.to_string(),
                    to: canonical.to_string(),
                },
                files_changed: Vec::new(),
            };
            let rendered = serde_json::to_string_pretty(&out)
                .map_err(|e| VdriftError::Internal(e.to_string()))?;
            println!("{rendered}");
        } else {
            println!("✓ already synchronized");
        }
        return Ok(0);
    }

    if cli.dry_run {
        if cli.json {
            let out = json::ApplyOutput {
                status: "dry_run".into(),
                version: json::VersionOut {
                    from: canonical.to_string(),
                    to: canonical.to_string(),
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

    let applied = sync_references(
        &repo,
        &detection.refs,
        &canonical,
        &config,
        false,
        args.force,
    )?;

    if cli.json {
        let out = json::ApplyOutput {
            status: "updated".into(),
            version: json::VersionOut {
                from: changes[0]
                    .from
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                to: canonical.to_string(),
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
        let ver = verification::verify(&detection.refs, Some(canonical.clone()));
        if ver.is_consistent() {
            println!();
            println!("✓ version consistency verified");
        }
    }

    Ok(0)
}
