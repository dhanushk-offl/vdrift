mod adapters;
mod cli;
mod config;
mod core;
mod errors;
mod git;
mod output;

use clap::Parser;
use errors::{Result, VdriftError};

fn main() {
    let cli = cli::Cli::parse();

    let exit_code = match cli::run(&cli) {
        Ok(code) => code,
        Err(err) => {
            report_error(&err, cli.json);
            err.exit_code()
        }
    };

    std::process::exit(exit_code);
}

fn report_error(err: &VdriftError, json: bool) {
    if json {
        let body = output::json::error_body(err.code(), &err.to_string());
        let rendered = serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".into());
        println!("{rendered}");
    } else {
        eprintln!("✗ {err}");
    }
}

/// Kept for library consumers; not used by the binary.
pub fn run_cli() -> Result<i32> {
    let cli = cli::Cli::parse();
    cli::run(&cli)
}
