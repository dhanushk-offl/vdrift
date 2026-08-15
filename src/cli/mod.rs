use crate::errors::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

pub mod apply;
pub mod bump;
pub mod check;
pub mod disable;
pub mod doctor;
pub mod hook;
pub mod init;
pub mod plan;
pub mod root;
pub mod scan;
pub mod status;
pub mod sync;
pub mod uninstall;
pub mod verify;

/// vdrift — version drift shouldn't happen.
#[derive(Debug, Parser)]
#[command(
    name = "vdrift",
    version,
    about = "Detect, propose, synchronize, and verify versions across your codebase.",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Machine-readable JSON output.
    #[arg(long, global = true)]
    pub json: bool,

    /// Show what would change without modifying anything.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Run as if started in this directory.
    #[arg(short = 'C', long, global = true)]
    pub dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Install the global Git integration (once per machine).
    Init,
    /// Show installation and integration status.
    Status,
    /// Remove the global integration and restore the previous Git configuration.
    Uninstall(UninstallArgs),
    /// Diagnose the installation and environment.
    Doctor,
    /// Disable vdrift for the current repository.
    Disable,
    /// Discover project and version information.
    Scan,
    /// Check for version drift.
    Check,
    /// Synchronize references to the canonical version.
    Sync(SyncArgs),
    /// Bump the version: patch | minor | major | <version>.
    Bump(BumpArgs),
    /// Produce a read-only version plan.
    Plan,
    /// Apply a version update (non-interactive).
    Apply(ApplyArgs),
    /// Verify version consistency.
    Verify(VerifyArgs),
    /// Internal Git hook entry point.
    Hook(HookArgs),
}

#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// Skip the confirmation prompt.
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Override the dirty-working-tree safety check.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct BumpArgs {
    /// patch, minor, major, or an explicit version like 2.0.0.
    pub level: String,

    /// Also create a commit with the version change.
    #[arg(long, conflicts_with = "no_commit")]
    pub commit: bool,

    /// Do not create a commit (default).
    #[arg(long, conflicts_with = "commit")]
    pub no_commit: bool,

    /// Override the dirty-working-tree safety check.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ApplyArgs {
    /// Target version to apply, e.g. 1.5.0.
    #[arg(long)]
    pub version: String,

    /// Also create a commit with the version change.
    #[arg(long, conflicts_with = "no_commit")]
    pub commit: bool,

    /// Do not create a commit (default).
    #[arg(long, conflicts_with = "commit")]
    pub no_commit: bool,

    /// Override the dirty-working-tree safety check.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// Minimal deterministic output for CI pipelines.
    #[arg(long)]
    pub ci: bool,
}

#[derive(Debug, Args)]
pub struct HookArgs {
    /// The Git hook phase: pre-push.
    pub phase: String,
}

pub fn run(cli: &Cli) -> Result<i32> {
    match &cli.command {
        None => root::run(cli),
        Some(Command::Init) => init::run(cli),
        Some(Command::Status) => status::run(cli),
        Some(Command::Uninstall(args)) => uninstall::run(cli, args),
        Some(Command::Doctor) => doctor::run(cli),
        Some(Command::Disable) => disable::run(cli),
        Some(Command::Scan) => scan::run(cli),
        Some(Command::Check) => check::run(cli),
        Some(Command::Sync(args)) => sync::run(cli, args),
        Some(Command::Bump(args)) => bump::run(cli, args),
        Some(Command::Plan) => plan::run(cli),
        Some(Command::Apply(args)) => apply::run(cli, args),
        Some(Command::Verify(args)) => verify::run(cli, args),
        Some(Command::Hook(args)) => hook::run(cli, args),
    }
}

pub(crate) fn repo(cli: &Cli) -> crate::git::repository::Repository {
    let dir = cli
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    crate::git::repository::Repository::discover(&dir)
}
