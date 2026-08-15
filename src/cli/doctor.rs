use crate::cli::Cli;
use crate::config::global;
use crate::config::global::GlobalConfig;
use crate::errors::Result;
use crate::git::hooks;
use std::process::Command;

pub fn run(_cli: &Cli) -> Result<i32> {
    let cfg = GlobalConfig::load()?;
    let hooks_dir = global::hooks_dir()?;
    let managed =
        hooks::current_hooks_path()?.as_deref() == Some(hooks_dir.to_string_lossy().as_ref());
    let dispatcher = hooks_dir.join(hooks::PRE_PUSH_HOOK).is_file();
    let config_ok = global::config_path().map(|p| p.is_file()).unwrap_or(false);

    println!("vdrift doctor");
    println!();

    println!("installation");
    println!();
    println!(
        "  ✓ binary available       {}",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".into())
    );
    let binary = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "?".into());
    println!("  ✓ binary available       {binary}");
    #[cfg(unix)]
    let executable = std::env::current_exe()
        .and_then(|p| {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&p).map(|m| m.permissions().mode() & 0o111 != 0)
        })
        .unwrap_or(true);
    #[cfg(not(unix))]
    let executable = true;
    println!(
        "  {} executable             {}",
        if executable { "✓" } else { "✗" },
        if executable { "yes" } else { "no" }
    );
    println!();

    println!("configuration");
    println!();
    println!(
        "  ✓ config directory      {}",
        hooks_dir.parent().unwrap_or(&hooks_dir).display()
    );
    println!(
        "  {} config file           {}",
        if config_ok { "✓" } else { "—" },
        if config_ok {
            "present"
        } else {
            "missing (run vdrift init)"
        }
    );
    println!();

    println!("git");
    println!();
    let git_ok = Command::new("git")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    println!(
        "  {} git available          {}",
        if git_ok { "✓" } else { "✗" },
        if git_ok { "yes" } else { "no" }
    );
    println!(
        "  {} global hooks path      {}",
        if managed { "✓" } else { "✗" },
        if managed {
            "managed by vdrift"
        } else {
            "not managed"
        }
    );
    println!(
        "  {} pre-push dispatcher    {}",
        if dispatcher { "✓" } else { "✗" },
        if dispatcher { "present" } else { "missing" }
    );
    println!();

    println!("environment");
    println!();
    println!("  OS            {}", std::env::consts::OS);
    println!("  architecture  {}", std::env::consts::ARCH);
    println!();

    println!("status");
    println!();
    let ready = managed && dispatcher && config_ok;
    if ready {
        println!("  ✓ ready");
    } else {
        println!("  ✗ incomplete — run: vdrift init");
    }
    println!();
    println!(
        "vdrift {}.",
        cfg.installed_version.as_deref().unwrap_or("—")
    );

    Ok(0)
}
