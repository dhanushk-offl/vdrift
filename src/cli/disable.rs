use crate::cli::Cli;
use crate::config::project::ProjectConfig;
use crate::errors::{Result, VdriftError};

pub fn run(cli: &Cli) -> Result<i32> {
    let repo = crate::cli::repo(cli);
    if !repo.is_git_repo() {
        return Err(VdriftError::Git(
            "disable only works inside a Git repository".into(),
        ));
    }

    let path: std::path::PathBuf = repo.root.join(".vdrift.toml");
    let mut config = ProjectConfig::load(&repo.root)?;
    config.behavior.enabled = Some(false);

    let rendered = toml::to_string(&config)
        .map_err(|e| VdriftError::Config(format!("failed to serialize config: {e}")))?;
    std::fs::write(&path, rendered)
        .map_err(|e| VdriftError::Config(format!("cannot write {}: {e}", path.display())))?;

    if cli.json {
        println!(
            "{}",
            serde_json::json!({ "status": "disabled", "file": ".vdrift.toml" })
        );
    } else {
        println!("✓ vdrift disabled for this repository");
        println!();
        println!("Wrote:");
        println!();
        println!("  .vdrift.toml");
        println!();
        println!("To re-enable, remove that file or set enabled = true.");
    }

    Ok(0)
}
