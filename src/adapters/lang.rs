use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// Adapter for lightweight ecosystem manifests that carry a single version
/// key: Dart/Flutter `pubspec.yaml`, Elixir `mix.exs`, PHP `composer.json`,
/// Ruby `*.gemspec`, and Haskell `package.yaml` / `*.cabal`.
pub struct LangAdapter;

const MIX_KEYS: &[&str] = &["version:"];
const GEMSPEC_KEYS: &[&str] = &[
    "s.version =",
    "spec.version =",
    "s.version=",
    "spec.version=",
];

impl super::VersionAdapter for LangAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();

        let pubspec = repo.root.join("pubspec.yaml");
        if pubspec.is_file() {
            let current = util::read_yaml_keys(&pubspec, &["version"])?;
            refs.push(VersionReference::new(
                pubspec,
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        let mix = repo.root.join("mix.exs");
        if mix.is_file() {
            let current = util::read_line_version(&mix, MIX_KEYS)?;
            refs.push(VersionReference::new(
                mix,
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        let composer = repo.root.join("composer.json");
        if composer.is_file() {
            let current = util::read_json_version(&composer)?;
            refs.push(VersionReference::new(
                composer,
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        let package_yaml = repo.root.join("package.yaml");
        if package_yaml.is_file() {
            let current = util::read_line_version(&package_yaml, MIX_KEYS)?;
            refs.push(VersionReference::new(
                package_yaml,
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        for name in root_files(repo) {
            if name.ends_with(".gemspec") {
                let path = repo.root.join(&name);
                let current = util::read_line_version(&path, GEMSPEC_KEYS)?;
                refs.push(VersionReference::new(
                    path,
                    current,
                    ReferenceKind::Canonical,
                    true,
                ));
            } else if name.ends_with(".cabal") {
                let path = repo.root.join(&name);
                let current = util::read_line_version(&path, MIX_KEYS)?;
                refs.push(VersionReference::new(
                    path,
                    current,
                    ReferenceKind::Canonical,
                    true,
                ));
            }
        }

        Ok(refs)
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        let name = reference
            .file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let old = reference.current.as_ref();
        match name {
            "pubspec.yaml" => util::write_yaml_keys(&reference.file, &["version"], version),
            "mix.exs" => write_keyed(reference, version, old, MIX_KEYS),
            "composer.json" => util::write_json_version(&reference.file, version),
            _ if name.ends_with(".gemspec") => write_keyed(reference, version, old, GEMSPEC_KEYS),
            _ if name.ends_with(".cabal") => write_keyed(reference, version, old, MIX_KEYS),
            "package.yaml" => write_keyed(reference, version, old, MIX_KEYS),
            _ => Err(VdriftError::Adapter(format!(
                "lang adapter cannot update {}",
                reference.file.display()
            ))),
        }
    }
}

fn write_keyed(
    reference: &VersionReference,
    version: &Version,
    old: Option<&Version>,
    keys: &[&str],
) -> Result<()> {
    if !util::write_line_version(&reference.file, keys, old, version)? {
        return Err(VdriftError::Adapter(format!(
            "no version assignment in {}",
            reference.file.display()
        )));
    }
    Ok(())
}

fn root_files(repo: &Repository) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(&repo.root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false)
            && let Some(name) = entry.file_name().to_str()
        {
            out.push(name.to_string());
        }
    }
    out
}
