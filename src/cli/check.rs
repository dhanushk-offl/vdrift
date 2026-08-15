use crate::cli::Cli;
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::core::verification;
use crate::errors::{Result, VdriftError};
use crate::output::{human, json};

pub fn run(cli: &Cli) -> Result<i32> {
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

    let ver = verification::verify(&detection.refs, Some(canonical.clone()));
    let consistent = ver.is_consistent();

    if cli.json {
        let canonical_file = detection
            .canonical_file
            .as_ref()
            .map(|f| {
                f.strip_prefix(&repo.root)
                    .unwrap_or(f)
                    .display()
                    .to_string()
            })
            .unwrap_or_default();
        let out = json::CheckOutput {
            status: if consistent {
                "valid".into()
            } else {
                "drift".into()
            },
            canonical: Some(json::CanonicalOut {
                file: canonical_file,
                version: canonical.to_string(),
            }),
            outdated: ver
                .drifts
                .iter()
                .map(|(file, v)| json::OutdatedOut {
                    file: file
                        .strip_prefix(&repo.root)
                        .unwrap_or(file)
                        .display()
                        .to_string(),
                    version: v.as_ref().map(|v| v.to_string()),
                })
                .collect(),
            count: ver.drifts.len(),
        };
        let rendered =
            serde_json::to_string_pretty(&out).map_err(|e| VdriftError::Internal(e.to_string()))?;
        println!("{rendered}");
    } else if consistent {
        human::ok();
    } else {
        human::drift_report(&detection, &repo.root);
    }

    Ok(if consistent { 0 } else { 1 })
}
