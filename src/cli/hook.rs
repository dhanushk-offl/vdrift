use crate::cli::{Cli, HookArgs};
use crate::errors::{Result, VdriftError};
use crate::git::dispatcher;

pub fn run(_cli: &Cli, args: &HookArgs) -> Result<i32> {
    match args.phase.as_str() {
        "pre-push" => dispatcher::run_pre_push(),
        other => Err(VdriftError::Config(format!(
            "unknown hook phase `{other}` — supported: pre-push"
        ))),
    }
}
