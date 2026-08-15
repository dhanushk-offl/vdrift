use crate::cli::Cli;
use crate::config::project::ProjectConfig;
use crate::core::detection;
use crate::core::proposal;
use crate::core::synchronization::plan_changes;
use crate::errors::{Result, VdriftError};
use crate::git::diff;
use crate::output::json::{self, PlanOutput};

pub fn run(cli: &Cli) -> Result<i32> {
    let repo = crate::cli::repo(cli);
    let config = ProjectConfig::load(&repo.root)?;
    let detection = detection::detect(&repo, &config)?;

    let Some(canonical) = detection.canonical.clone() else {
        let out = PlanOutput {
            status: "no_project".into(),
            project: detection.project.clone(),
            current_version: None,
            suggested_version: None,
            reason: json::ReasonOut {
                r#type: "none",
                confidence: 0.0,
            },
            changes: Vec::new(),
        };
        if cli.json {
            let rendered = serde_json::to_string_pretty(&out)
                .map_err(|e| VdriftError::Internal(e.to_string()))?;
            println!("{rendered}");
        } else {
            println!("status: no_project");
            println!("No version detected in this project.");
        }
        return Ok(0);
    };

    let messages = if repo.is_git_repo() {
        let baseline = detection
            .canonical_file
            .as_ref()
            .and_then(|f| diff::last_commit_for(&repo, f));
        diff::messages_since(&repo, baseline.as_deref())
    } else {
        Vec::new()
    };

    let proposal = proposal::analyze(&messages);
    let target = canonical.bump(proposal.level);
    let drifted = detection.drifted_refs();

    let status = if proposal.needs_update() {
        "needs_update"
    } else if !drifted.is_empty() {
        "drift"
    } else {
        "synchronized"
    };

    let changes = plan_changes(&detection, &target);
    let changes_out = changes
        .iter()
        .map(|c| json::ChangeOut::from_change(c, &repo.root))
        .collect();

    let reason_type = match proposal.reason {
        crate::core::proposal::ReasonType::Major => "major",
        crate::core::proposal::ReasonType::Feature => "feature",
        crate::core::proposal::ReasonType::Fix => "fix",
        crate::core::proposal::ReasonType::None => "none",
    };

    let out = PlanOutput {
        status: status.to_string(),
        project: detection.project.clone(),
        current_version: Some(canonical.to_string()),
        suggested_version: if proposal.needs_update() {
            Some(target.to_string())
        } else {
            None
        },
        reason: json::ReasonOut {
            r#type: reason_type,
            confidence: proposal.confidence,
        },
        changes: changes_out,
    };

    if cli.json {
        let rendered =
            serde_json::to_string_pretty(&out).map_err(|e| VdriftError::Internal(e.to_string()))?;
        println!("{rendered}");
    } else {
        println!("status: {}", out.status);
        if let Some(v) = &out.current_version {
            println!("current: {v}");
        }
        if let Some(v) = &out.suggested_version {
            println!("suggested: {v}");
        }
        println!(
            "reason: {} (confidence {:.2})",
            out.reason.r#type, out.reason.confidence
        );
        for c in &out.changes {
            let from = c.from.clone().unwrap_or_else(|| "—".into());
            println!("  {} {from} → {}", c.file, c.to);
        }
    }

    Ok(0)
}
