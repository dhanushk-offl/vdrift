use crate::cli::Cli;
use crate::config::global;
use crate::config::global::GlobalConfig;
use crate::errors::Result;
use crate::git::hooks;

pub fn run(_cli: &Cli) -> Result<i32> {
    let hooks_dir = global::hooks_dir()?;
    let mut cfg = GlobalConfig::load()?;

    if !cfg.is_installed() {
        // Record any pre-existing global hooks path so uninstall can restore it.
        if let Some(current) = hooks::current_hooks_path()?
            && current != hooks_dir.to_string_lossy()
        {
            cfg.previous_hooks_path = Some(current);
        }
        cfg.hooks_path = Some(hooks_dir.to_string_lossy().to_string());
        cfg.enabled = true;
        cfg.installed_version = Some(env!("CARGO_PKG_VERSION").into());
    }

    hooks::write_dispatcher(&hooks_dir)?;

    if hooks::current_hooks_path()?.as_deref() != Some(hooks_dir.to_string_lossy().as_ref()) {
        hooks::set_hooks_path(&hooks_dir.to_string_lossy())?;
    }

    cfg.save()?;

    println!("vdrift global setup");
    println!();
    println!("  ✓ configuration directory");
    println!("  ✓ Git integration");
    println!("  ✓ global pre-push dispatcher");
    println!();
    println!("vdrift is now active globally.");
    println!();
    println!("You don't need to initialize individual repositories.");
    println!();
    println!("Try:");
    println!();
    println!("  cd any/git/repository");
    println!("  git push");

    Ok(0)
}
