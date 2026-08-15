use crate::cli::{Cli, UninstallArgs};
use crate::config::global;
use crate::config::global::GlobalConfig;
use crate::errors::{Result, VdriftError};
use crate::git::hooks;
use std::io::IsTerminal;

pub fn run(_cli: &Cli, args: &UninstallArgs) -> Result<i32> {
    let cfg = GlobalConfig::load()?;
    let hooks_dir = global::hooks_dir()?;

    let confirmed = if args.yes {
        true
    } else {
        if !std::io::stdin().is_terminal() && !std::io::stderr().is_terminal() {
            return Err(VdriftError::Cancelled(
                "uninstall requires an interactive terminal (or --yes)".into(),
            ));
        }
        println!("Remove global vdrift integration?");
        println!();
        println!("This will remove:");
        println!();
        println!("  {}", hooks_dir.display());
        println!();

        let term = dialoguer::console::Term::stderr();
        dialoguer::Confirm::new()
            .with_prompt("Continue? [y/N]")
            .default(false)
            .interact_on(&term)
            .map_err(|e| VdriftError::Cancelled(e.to_string()))?
    };

    if !confirmed {
        println!("Aborted.");
        return Ok(0);
    }

    // Restore the previous Git hook configuration.
    if let Some(prev) = cfg.previous_hooks_path.clone() {
        hooks::set_hooks_path(&prev)?;
    } else {
        hooks::unset_hooks_path()?;
    }

    // Remove the dispatcher.
    hooks::remove_dispatcher(&hooks_dir)?;
    let _ = std::fs::remove_dir(&hooks_dir);

    // Remove the global config file and empty parent directory.
    if let Ok(config_path) = global::config_path() {
        let _ = std::fs::remove_file(&config_path);
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    println!("vdrift has been removed.");
    println!("The previous Git hook configuration was restored.");

    Ok(0)
}
