use crate::cli::Cli;
use crate::config::global;
use crate::config::global::GlobalConfig;
use crate::errors::Result;
use crate::git::hooks;

pub fn run(_cli: &Cli) -> Result<i32> {
    let cfg = GlobalConfig::load()?;
    let hooks_dir = global::hooks_dir()?;
    let current = hooks::current_hooks_path()?;
    let managed = current.as_deref() == Some(hooks_dir.to_string_lossy().as_ref());
    let dispatcher = hooks_dir.join(hooks::PRE_PUSH_HOOK).is_file();

    println!("vdrift");
    println!();

    println!("installation");
    println!();
    println!(
        "  version      {}",
        cfg.installed_version.as_deref().unwrap_or("—")
    );
    let binary = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "—".into());
    println!("  binary       {binary}");
    println!();

    println!("global integration");
    println!();
    println!(
        "  enabled      {}",
        if cfg.enabled {
            "✓".to_string()
        } else {
            "✗".to_string()
        }
    );
    println!();

    println!("git");
    println!();
    println!("  hooks path   {}", current.as_deref().unwrap_or("(none)"));
    println!(
        "  pre-push     {}",
        if dispatcher {
            "✓".to_string()
        } else {
            "✗".to_string()
        }
    );
    println!();

    println!("configuration");
    println!();
    println!(
        "  valid        {}",
        if cfg.hooks_path.is_some() {
            "✓".to_string()
        } else {
            "—".to_string()
        }
    );

    if !cfg.enabled || !managed || !dispatcher {
        println!();
        println!("Run:");
        println!();
        println!("  vdrift init");
    }

    Ok(0)
}
