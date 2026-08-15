use crate::cli::Cli;
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::errors::{Result, VdriftError};
use crate::output::{human, json::ScanOutput};

pub fn run(cli: &Cli) -> Result<i32> {
    let repo = crate::cli::repo(cli);
    let config = ProjectConfig::load(&repo.root)?;
    let detection = detection::detect(&repo, &config)?;

    let candidates = match &detection.canonical {
        Some(canon) => detection::scan_candidates(&repo, canon),
        None => Vec::new(),
    };

    if cli.json {
        let candidate_out = candidates
            .iter()
            .map(|r| crate::output::json::SourceOut {
                file: r
                    .file
                    .strip_prefix(&repo.root)
                    .unwrap_or(&r.file)
                    .display()
                    .to_string(),
                version: r.current.as_ref().map(|v| v.to_string()),
                kind: r.kind.label(),
                writable: r.writable,
            })
            .collect();
        let out = ScanOutput::from_detection(
            &detection,
            candidate_out,
            &repo.root,
            detection.canonical.as_ref(),
        );
        let rendered =
            serde_json::to_string_pretty(&out).map_err(|e| VdriftError::Internal(e.to_string()))?;
        println!("{rendered}");
    } else {
        human::scan(&detection, &candidates, &repo.root);
    }

    Ok(0)
}
