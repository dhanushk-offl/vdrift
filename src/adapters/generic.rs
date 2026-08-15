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
            if !matches!(name.as_str(), "package.json" | "Cargo.toml") {
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
            let current = read_version(&path)?;
            refs.push(VersionReference::new(path, current, kind, true));
        }

        Ok(refs)
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        update_file(&reference.file, reference.current.as_ref(), version)
    }
}

/// Reads the top-level `version` value from a structured file, or `None` for
/// text files (matching happens later against the canonical version).
fn read_version(path: &Path) -> Result<Option<Version>> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Ok(None);
    };
    if !matches!(ext, "json" | "yaml" | "yml" | "toml") {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))?;

    let value: Option<String> = match ext {
        "json" => serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|e| VdriftError::Adapter(format!("invalid JSON in {}: {e}", path.display())))?
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        "yaml" | "yml" => serde_yaml::from_str::<serde_yaml::Value>(&text)
            .map_err(|e| VdriftError::Adapter(format!("invalid YAML in {}: {e}", path.display())))?
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        "toml" => toml::from_str::<toml::Value>(&text)
            .map_err(|e| VdriftError::Adapter(format!("invalid TOML in {}: {e}", path.display())))?
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        _ => None,
    };

    match value {
        Some(s) => Ok(Some(Version::parse(&s)?)),
        None => Ok(None),
    }
}

fn update_file(path: &Path, old: Option<&Version>, version: &Version) -> Result<()> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return update_text(path, old, version);
    };

    let text = std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))?;

    let rendered: String = match ext {
        "json" => {
            let mut value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                VdriftError::Adapter(format!("invalid JSON in {}: {e}", path.display()))
            })?;
            if !value.is_object() {
                return Err(VdriftError::Adapter(format!(
                    "{} is not a JSON object",
                    path.display()
                )));
            }
            value["version"] = serde_json::Value::String(version.to_string());
            serde_json::to_string_pretty(&value).map_err(|e| {
                VdriftError::Adapter(format!("failed to serialize {}: {e}", path.display()))
            })?
        }
        "yaml" | "yml" => {
            let mut value: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| {
                VdriftError::Adapter(format!("invalid YAML in {}: {e}", path.display()))
            })?;
            value["version"] = serde_yaml::Value::String(version.to_string());
            serde_yaml::to_string(&value).map_err(|e| {
                VdriftError::Adapter(format!("failed to serialize {}: {e}", path.display()))
            })?
        }
        "toml" => {
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
            toml::to_string(&value).map_err(|e| {
                VdriftError::Adapter(format!("failed to serialize {}: {e}", path.display()))
            })?
        }
        _ => return update_text(path, old, version),
    };

    std::fs::write(path, rendered)
        .map_err(|e| VdriftError::Adapter(format!("cannot write {}: {e}", path.display())))
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

    let text = std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))?;
    if !text.contains(&old_str) {
        return Err(VdriftError::Adapter(format!(
            "{} does not contain version {old_str}",
            path.display()
        )));
    }
    let updated = text.replace(&old_str, &new_str);
    std::fs::write(path, updated)
        .map_err(|e| VdriftError::Adapter(format!("cannot write {}: {e}", path.display())))
}
