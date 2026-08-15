use crate::adapters::is_known_manifest;
use crate::adapters::util;
use crate::config::project::ProjectConfig;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;
use std::path::Path;

/// Config-driven adapter for arbitrary JSON / YAML / TOML / text references.
///
/// Files listed in `[references] files` (and the configured `[version] source`
/// when it isn't a known manifest) are detected as writable references. Files
/// found only by generic string scanning are never writable.
pub struct GenericAdapter {
    config: ProjectConfig,
}

impl GenericAdapter {
    pub fn new(config: ProjectConfig) -> Self {
        GenericAdapter { config }
    }

    /// File paths this adapter owns, relative to the repo root.
    fn owned_files(&self) -> Vec<String> {
        let mut files: Vec<String> = self.config.reference_files().to_vec();
        if let Some(source) = self.config.version_source() {
            let name = Path::new(&source)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            // Known manifests are owned by their ecosystem adapter.
            if !is_known_manifest(&name) {
                files.push(source);
            }
        }
        files
    }
}

impl super::VersionAdapter for GenericAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();
        let source = self.config.version_source();

        for rel in self.owned_files() {
            let path = repo.root.join(&rel);
            if !path.is_file() {
                continue;
            }
            let kind = if source.as_deref() == Some(rel.as_str()) {
                ReferenceKind::Canonical
            } else {
                ReferenceKind::Reference
            };
            let current = read_structured_version(&path)?;
            refs.push(VersionReference::new(path, current, kind, true));
        }

        Ok(refs)
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        update_file(&reference.file, reference.current.as_ref(), version)
    }
}

/// Reads a self-contained version for structured files; `Ok(None)` for
/// plain text (matched later against the canonical version).
fn read_structured_version(path: &Path) -> Result<Option<Version>> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Ok(None);
    };
    match ext {
        "json" => util::read_json_version(path),
        "yaml" | "yml" => util::read_yaml_keys(path, &["version"]),
        "toml" => util::read_toml_keys(path, &["version"]),
        _ => Ok(None),
    }
}

fn update_file(path: &Path, old: Option<&Version>, version: &Version) -> Result<()> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return update_text(path, old, version);
    };
    match ext {
        "json" => util::write_json_version(path, version),
        "yaml" | "yml" => util::write_yaml_keys(path, &["version"], version),
        "toml" => {
            let text = util::read_text(path)?;
            if util::write_section_keys(path, "version", &["version =", "version="], old, version)?
            {
                return Ok(());
            }
            // No version line; fall back to structured serialization.
            let mut value: toml::Value = toml::from_str(&text).map_err(|e| {
                VdriftError::Adapter(format!("invalid TOML in {}: {e}", path.display()))
            })?;
            if !value.is_table() {
                return Err(VdriftError::Adapter(format!(
                    "{} is not a TOML table",
                    path.display()
                )));
            }
            value["version"] = toml::Value::String(version.to_string());
            let rendered = toml::to_string(&value).map_err(|e| {
                VdriftError::Adapter(format!("failed to serialize {}: {e}", path.display()))
            })?;
            util::write_text(path, &rendered)
        }
        _ => update_text(path, old, version),
    }
}

/// Replaces exact occurrences of the old version string in plain text.
fn update_text(path: &Path, old: Option<&Version>, version: &Version) -> Result<()> {
    let Some(old) = old else {
        return Err(VdriftError::Adapter(format!(
            "cannot update {}: the current version is unknown",
            path.display()
        )));
    };
    let old_str = old.to_string();
    let new_str = version.to_string();

    let text = util::read_text(path)?;
    if !text.contains(&old_str) {
        return Err(VdriftError::Adapter(format!(
            "{} does not contain version {old_str}",
            path.display()
        )));
    }
    let updated = text.replace(&old_str, &new_str);
    util::write_text(path, &updated)
}
