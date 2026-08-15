use crate::config::project::ProjectConfig;
use crate::core::detection::VersionReference;
use crate::core::version::Version;
use crate::errors::Result;
use crate::git::repository::Repository;

pub mod cargo;
pub mod generic;
pub mod npm;

/// An ecosystem adapter knows how to read and write version references for a
/// specific package format. The core engine has no format-specific knowledge.
pub trait VersionAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>>;

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()>;
}

/// All auto-detection adapters, in canonical-priority order.
pub fn all() -> Vec<Box<dyn VersionAdapter>> {
    vec![Box::new(npm::NpmAdapter), Box::new(cargo::CargoAdapter)]
}

/// Config-driven adapter for extra/custom reference files.
pub fn generic(config: &ProjectConfig) -> Box<dyn VersionAdapter> {
    Box::new(generic::GenericAdapter::new(config.clone()))
}

/// Resolves the adapter that owns a given file (used during update).
pub fn adapter_for(file: &std::path::Path, config: &ProjectConfig) -> Box<dyn VersionAdapter> {
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match name {
        "package.json" | "package-lock.json" => Box::new(npm::NpmAdapter),
        "Cargo.toml" | "Cargo.lock" => Box::new(cargo::CargoAdapter),
        _ => generic(config),
    }
}
