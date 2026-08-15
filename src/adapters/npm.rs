use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

pub struct NpmAdapter;

impl super::VersionAdapter for NpmAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();

        let manifest = repo.root.join("package.json");
        if manifest.is_file() {
            let text = std::fs::read_to_string(&manifest).map_err(|e| {
                VdriftError::Adapter(format!("cannot read {}: {e}", manifest.display()))
            })?;
            let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                VdriftError::Adapter(format!("invalid JSON in {}: {e}", manifest.display()))
            })?;
            let version = value
                .get("version")
                .and_then(|v| v.as_str())
                .map(Version::parse)
                .transpose()?;
            refs.push(VersionReference::new(
                manifest,
                version,
                ReferenceKind::Canonical,
                true,
            ));
        }

        let lock = repo.root.join("package-lock.json");
        if lock.is_file() {
            let text = std::fs::read_to_string(&lock).map_err(|e| {
                VdriftError::Adapter(format!("cannot read {}: {e}", lock.display()))
            })?;
            let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                VdriftError::Adapter(format!("invalid JSON in {}: {e}", lock.display()))
            })?;
            let version = lock_version(&value).map(Version::parse).transpose()?;
            refs.push(VersionReference::new(
                lock,
                version,
                ReferenceKind::Derived,
                true,
            ));
        }

        Ok(refs)
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        let name = reference
            .file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        match name {
            "package.json" => update_manifest(&reference.file, version),
            "package-lock.json" => update_lock(&reference.file, version),
            _ => Err(VdriftError::Adapter(format!(
                "npm adapter cannot update {}",
                reference.file.display()
            ))),
        }
    }
}

fn lock_version(value: &serde_json::Value) -> Option<&str> {
    if let Some(v) = value
        .get("packages")
        .and_then(|p| p.get(""))
        .and_then(|root| root.get("version"))
        .and_then(|v| v.as_str())
    {
        return Some(v);
    }
    value.get("version").and_then(|v| v.as_str())
}

fn write_json(path: &std::path::Path, value: &serde_json::Value) -> Result<()> {
    let rendered = serde_json::to_string_pretty(value).map_err(|e| {
        VdriftError::Adapter(format!("failed to serialize {}: {e}", path.display()))
    })?;
    std::fs::write(path, rendered + "\n")
        .map_err(|e| VdriftError::Adapter(format!("cannot write {}: {e}", path.display())))
}

fn update_manifest(path: &std::path::Path, version: &Version) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))?;
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| VdriftError::Adapter(format!("invalid JSON in {}: {e}", path.display())))?;
    if !value.is_object() {
        return Err(VdriftError::Adapter(format!(
            "{} is not a JSON object",
            path.display()
        )));
    }
    value["version"] = serde_json::Value::String(version.to_string());
    write_json(path, &value)
}

fn update_lock(path: &std::path::Path, version: &Version) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))?;
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| VdriftError::Adapter(format!("invalid JSON in {}: {e}", path.display())))?;

    if let Some(packages) = value.get_mut("packages").and_then(|p| p.as_object_mut())
        && let Some(root) = packages.get_mut("")
        && root.is_object()
        && root.get("version").is_some()
    {
        root["version"] = serde_json::Value::String(version.to_string());
    }
    if let Some(root) = value.as_object_mut()
        && root.get("version").is_some()
    {
        root["version"] = serde_json::Value::String(version.to_string());
    }

    write_json(path, &value)
}
