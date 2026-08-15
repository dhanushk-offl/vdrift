use crate::config::project::ProjectConfig;
use crate::core::detection::VersionReference;
use crate::core::version::Version;
use crate::errors::Result;
use crate::git::repository::Repository;

pub mod bun;
pub mod cargo;
pub mod generic;
pub mod go;
pub mod java;
pub mod lang;
pub mod npm;
pub mod pnpm;
pub mod python;
pub mod tauri;
pub mod util;
pub mod yarn;

/// An ecosystem adapter knows how to read and write version references for a
/// specific package format. The core engine has no format-specific knowledge.
pub trait VersionAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>>;

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()>;
}

/// All auto-detection adapters. Order determines which adapter owns a file
/// when two adapters would both claim it.
pub fn all() -> Vec<Box<dyn VersionAdapter>> {
    vec![
        Box::new(npm::NpmAdapter),
        Box::new(cargo::CargoAdapter),
        Box::new(tauri::TauriAdapter),
        Box::new(go::GoAdapter),
        Box::new(python::PythonAdapter),
        Box::new(java::JavaAdapter),
        Box::new(pnpm::PnpmAdapter),
        Box::new(bun::BunAdapter),
        Box::new(yarn::YarnAdapter),
        Box::new(lang::LangAdapter),
    ]
}

/// Config-driven adapter for extra/custom reference files.
pub fn generic(config: &ProjectConfig) -> Box<dyn VersionAdapter> {
    Box::new(generic::GenericAdapter::new(config.clone()))
}

/// True when a file is owned by an ecosystem adapter (so the generic adapter
/// must not claim it as a config-driven source).
pub fn is_known_manifest(name: &str) -> bool {
    matches!(
        name,
        "package.json"
            | "package-lock.json"
            | "Cargo.toml"
            | "Cargo.lock"
            | "tauri.conf.json"
            | "go.mod"
            | "version.go"
            | "pyproject.toml"
            | "setup.py"
            | "setup.cfg"
            | "__init__.py"
            | "_version.py"
            | "pnpm-lock.yaml"
            | "bun.lock"
            | "yarn.lock"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "gradle.properties"
            | "pubspec.yaml"
            | "mix.exs"
            | "composer.json"
            | "package.yaml"
    ) || name.ends_with(".gemspec")
        || name.ends_with(".cabal")
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
        "tauri.conf.json" => Box::new(tauri::TauriAdapter),
        "version.go" => Box::new(go::GoAdapter),
        "pyproject.toml" | "setup.py" | "setup.cfg" | "__init__.py" | "_version.py" => {
            Box::new(python::PythonAdapter)
        }
        "pom.xml" | "build.gradle" | "build.gradle.kts" | "gradle.properties" => {
            Box::new(java::JavaAdapter)
        }
        "pnpm-lock.yaml" => Box::new(pnpm::PnpmAdapter),
        "bun.lock" => Box::new(bun::BunAdapter),
        "yarn.lock" => Box::new(yarn::YarnAdapter),
        "pubspec.yaml" | "mix.exs" | "composer.json" | "package.yaml" => {
            Box::new(lang::LangAdapter)
        }
        _ if name.ends_with(".gemspec") || name.ends_with(".cabal") => Box::new(lang::LangAdapter),
        _ => generic(config),
    }
}
