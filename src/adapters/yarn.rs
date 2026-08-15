use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// Yarn lockfile adapter. Yarn lockfiles only carry a version for the root
/// project when workspaces list it explicitly; a derived reference is emitted
/// in that case and nothing otherwise.
pub struct YarnAdapter;

impl super::VersionAdapter for YarnAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let lock = repo.root.join("yarn.lock");
        let manifest = repo.root.join("package.json");
        if !lock.is_file() || !manifest.is_file() {
            return Ok(Vec::new());
        }
        let Some(name) = manifest_name(&manifest) else {
            return Ok(Vec::new());
        };
        if let Some(current) = find_root_version(&lock, &name)? {
            return Ok(vec![VersionReference::new(
                lock,
                Some(current),
                ReferenceKind::Derived,
                true,
            )]);
        }
        Ok(Vec::new())
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        if reference.file.file_name().and_then(|n| n.to_str()) != Some("yarn.lock") {
            return Err(VdriftError::Adapter(format!(
                "yarn adapter cannot update {}",
                reference.file.display()
            )));
        }
        let Some(name) = manifest_name(&reference.file.with_file_name("package.json")) else {
            return Err(VdriftError::Adapter(format!(
                "cannot update {}: package.json has no name",
                reference.file.display()
            )));
        };
        let text = util::read_text(&reference.file)?;
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let new_str = version.to_string();
        let old_str = reference.current.as_ref().map(|v| v.to_string());

        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            let is_root = trimmed.starts_with(&format!("\"{name}@"))
                || trimmed.starts_with(&format!("{name}@"));
            if !is_root {
                i += 1;
                continue;
            }
            // Entry body: indented lines up to the next top-level entry.
            let mut j = i + 1;
            while j < lines.len() && (lines[j].starts_with(' ') || lines[j].starts_with('\t')) {
                let inner = lines[j].trim();
                if inner.starts_with("version \"")
                    && let Some(old) = &old_str
                    && inner.contains(old)
                {
                    let indent: String = lines[j]
                        .chars()
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .collect();
                    lines[j] = format!("{indent}version \"{new_str}\"");
                    util::write_text(&reference.file, &(lines.join("\n") + "\n"))?;
                    return Ok(());
                }
                j += 1;
            }
            i = j;
        }
        Err(VdriftError::Adapter(format!(
            "no root package entry for {name} in {}",
            reference.file.display()
        )))
    }
}

fn manifest_name(manifest: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
}

fn find_root_version(lock: &std::path::Path, name: &str) -> Result<Option<Version>> {
    let text = util::read_text(lock)?;
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        let is_root =
            trimmed.starts_with(&format!("\"{name}@")) || trimmed.starts_with(&format!("{name}@"));
        if is_root {
            let mut j = i + 1;
            while j < lines.len() && (lines[j].starts_with(' ') || lines[j].starts_with('\t')) {
                let inner = lines[j].trim();
                if let Some(rest) = inner.strip_prefix("version ") {
                    let cleaned = rest.trim().trim_matches(|c: char| matches!(c, '"' | '\''));
                    if let Ok(v) = Version::parse(cleaned) {
                        return Ok(Some(v));
                    }
                }
                j += 1;
            }
        }
        i += 1;
    }
    Ok(None)
}
