use crate::cli::{Cli, VerifyArgs};
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::core::verification;
use crate::errors::{Result, VdriftError};
use crate::output::{human, json};

pub fn run(cli: &Cli, args: &VerifyArgs) -> Result<i32> {
    let repo = crate::cli::repo(cli);
    let config = ProjectConfig::load(&repo.root)?;
    let detection = detection::detect(&repo, &config)?;

    let Some(canonical) = detection.canonical.clone() else {
        return Err(VdriftError::Unsupported(
            "No canonical version source detected in this project.".into(),
        ));
    };

    let ver = verification::verify(&detection.refs, Some(canonical.clone()));
    let consistent = ver.is_consistent();

    if cli.json {
        let out = json::VerifyOutput {
            status: if consistent {
                "consistent".into()
            } else {
                "drift".into()
            },
            expected: Some(canonical.to_string()),
            drifts: ver
                .drifts
                .iter()
                .map(|(file, v)| json::DriftOut {
                    file: file
                        .strip_prefix(&repo.root)
                        .unwrap_or(file)
                        .display()
                        .to_string(),
                    found: v.as_ref().map(|v| v.to_string()),
                })
                .collect(),
        };
        let rendered =
            serde_json::to_string_pretty(&out).map_err(|e| VdriftError::Internal(e.to_string()))?;
        println!("{rendered}");
    } else if args.ci {
        if consistent {
            println!("✓ version consistent");
        } else {
            println!("✗ version drift detected");
            println!();
            println!("Expected: {canonical}");
            for (file, v) in &ver.drifts {
                let path = file
                    .strip_prefix(&repo.root)
                    .unwrap_or(file)
                    .display()
                    .to_string();
                let found = v
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into());
                println!("{path} → {found}");
            }
        }
    } else {
        if consistent {
            human::ok();
        } else {
            human::drift_report(&detection, &repo.root);
        }
    }

    Ok(if consistent { 0 } else { 1 })
}
