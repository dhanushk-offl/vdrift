use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// Bun lockfile adapter. The text `bun.lock` stores the root workspace under
/// `workspaces.""` including a `version`. The binary `bun.lockb` is ignored.
pub struct BunAdapter;

impl super::VersionAdapter for BunAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let lock = repo.root.join("bun.lock");
        if !lock.is_file() {
            return Ok(Vec::new());
        }
        let current = read_root_version(&lock)?;
        match current {
            Some(v) => Ok(vec![VersionReference::new(
                lock,
                Some(v),
                ReferenceKind::Derived,
                true,
            )]),
            None => Ok(Vec::new()),
        }
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        if reference.file.file_name().and_then(|n| n.to_str()) != Some("bun.lock") {
            return Err(VdriftError::Adapter(format!(
                "bun adapter cannot update {}",
                reference.file.display()
            )));
        }
        let text = util::read_text(&reference.file)?;
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(mut value) => {
                if let Some(ws) = value
                    .get_mut("workspaces")
                    .and_then(|w| w.as_object_mut())
                    .and_then(|w| w.get_mut(""))
                {
                    ws["version"] = serde_json::Value::String(version.to_string());
                } else {
                    return Err(VdriftError::Adapter(format!(
                        "no root workspace version in {}",
                        reference.file.display()
                    )));
                }
                let rendered = serde_json::to_string_pretty(&value).map_err(|e| {
                    VdriftError::Adapter(format!(
                        "failed to serialize {}: {e}",
                        reference.file.display()
                    ))
                })?;
                util::write_text(&reference.file, &(rendered + "\n"))
            }
            Err(_) => {
                // Fallback: JSON5-style lockfile; replace the first version pair.
                let old = reference.current.as_ref().map(|v| v.to_string());
                let Some(old) = old else {
                    return Err(VdriftError::Adapter(format!(
                        "cannot update {}: current version unknown",
                        reference.file.display()
                    )));
                };
                let needle = format!("\"version\": \"{old}\"");
                let replacement = format!("\"version\": \"{}\"", version);
                if !text.contains(&needle) {
                    return Err(VdriftError::Adapter(format!(
                        "{} does not contain version {old}",
                        reference.file.display()
                    )));
                }
                util::write_text(&reference.file, &text.replace(&needle, &replacement))
            }
        }
    }
}

fn read_root_version(path: &std::path::Path) -> Result<Option<Version>> {
    let text = util::read_text(path)?;
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        return Ok(value
            .get("workspaces")
            .and_then(|w| w.get(""))
            .and_then(|root| root.get("version"))
            .and_then(|v| v.as_str())
            .and_then(|s| Version::parse(s).ok()));
    }
    // JSON5-style: find the first `"version": "x.y.z"` pair (workspaces lead).
    for word in text.split_whitespace() {
        if let Some(rest) = word.strip_prefix("\"version\":") {
            let cleaned = rest.trim_matches(|c: char| matches!(c, '"' | ','));
            if let Ok(v) = Version::parse(cleaned) {
                return Ok(Some(v));
            }
        }
    }
    Ok(None)
}
