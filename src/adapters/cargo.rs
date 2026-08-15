use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

pub struct CargoAdapter;

impl super::VersionAdapter for CargoAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();
        let mut package_name: Option<String> = None;

        let manifest = repo.root.join("Cargo.toml");
        if manifest.is_file() {
            let text = std::fs::read_to_string(&manifest).map_err(|e| {
                VdriftError::Adapter(format!("cannot read {}: {e}", manifest.display()))
            })?;
            let value: toml::Value = toml::from_str(&text).map_err(|e| {
                VdriftError::Adapter(format!("invalid TOML in {}: {e}", manifest.display()))
            })?;
            let pkg = value.get("package").and_then(|p| p.as_table());
            let name = pkg
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let version = pkg
                .and_then(|p| p.get("version"))
                .and_then(|v| v.as_str())
                .map(Version::parse)
                .transpose()?;
            package_name = name.clone();
            refs.push(VersionReference::new(
                manifest,
                version,
                ReferenceKind::Canonical,
                true,
            ));
        }

        let lock = repo.root.join("Cargo.lock");
        if lock.is_file() {
            let text = std::fs::read_to_string(&lock).map_err(|e| {
                VdriftError::Adapter(format!("cannot read {}: {e}", lock.display()))
            })?;
            let value: toml::Value = toml::from_str(&text).map_err(|e| {
                VdriftError::Adapter(format!("invalid TOML in {}: {e}", lock.display()))
            })?;
            let version = lock_version(&value, package_name.as_deref())
                .map(Version::parse)
                .transpose()?;
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
        let old = reference.current.as_ref().map(|v| v.to_string());
        match name {
            "Cargo.toml" => update_manifest(&reference.file, version, old.as_deref()),
            "Cargo.lock" => update_lock(&reference.file, version, old.as_deref()),
            _ => Err(VdriftError::Adapter(format!(
                "cargo adapter cannot update {}",
                reference.file.display()
            ))),
        }
    }
}

fn lock_version<'a>(value: &'a toml::Value, package_name: Option<&str>) -> Option<&'a str> {
    let packages = value.get("package")?.as_array()?;
    for item in packages {
        let table = item.as_table()?;
        if let Some(name) = package_name {
            if table.get("name").and_then(|v| v.as_str()) == Some(name) {
                return table.get("version").and_then(|v| v.as_str());
            }
        } else if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
            return Some(v);
        }
    }
    None
}

/// Replaces the version value on a `version = "..."` line inside the section
/// delimited by [start, end) of `lines`.
fn replace_section_version(
    lines: &mut [String],
    start: usize,
    end: usize,
    old: Option<&str>,
    new: &str,
) -> bool {
    for i in start..end.min(lines.len()) {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("version") && trimmed.starts_with("version =") {
            if let Some(old) = old {
                let expected = format!("version = \"{old}\"");
                if trimmed != expected {
                    continue;
                }
            }
            let indent = lines[i]
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect::<String>();
            lines[i] = format!("{indent}version = \"{new}\"");
            return true;
        }
    }
    false
}

fn update_manifest(path: &std::path::Path, version: &Version, old: Option<&str>) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();

    let pkg_start = lines
        .iter()
        .position(|l| l.trim() == "[package]")
        .ok_or_else(|| {
            VdriftError::Adapter(format!("no [package] section in {}", path.display()))
        })?;
    let pkg_end = lines[pkg_start + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .map(|i| pkg_start + 1 + i)
        .unwrap_or(lines.len());

    if !replace_section_version(
        &mut lines,
        pkg_start + 1,
        pkg_end,
        old,
        &version.to_string(),
    ) {
        return Err(VdriftError::Adapter(format!(
            "no version line in [package] of {}",
            path.display()
        )));
    }

    let rendered = lines.join("\n") + "\n";
    std::fs::write(path, rendered)
        .map_err(|e| VdriftError::Adapter(format!("cannot write {}: {e}", path.display())))
}

fn update_lock(path: &std::path::Path, version: &Version, old: Option<&str>) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let new = version.to_string();

    // Determine the root package name from the sibling Cargo.toml so we only
    // touch its [[package]] block in the lockfile.
    let root_name = sibling_crate_name(path)?;
    let mut changed = false;

    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() != "[[package]]" {
            i += 1;
            continue;
        }
        let block_start = i;
        let mut block_end = i + 1;
        while block_end < lines.len() && lines[block_end].trim() != "[[package]]" {
            block_end += 1;
        }

        let name_line = lines[block_start..block_end]
            .iter()
            .find(|l| l.trim_start().starts_with("name ="))
            .map(|l| l.trim().to_string());
        let matches = match (&root_name, name_line) {
            (Some(root), Some(nl)) => nl == format!("name = \"{root}\""),
            (None, _) => true,
            _ => false,
        };

        if matches && replace_section_version(&mut lines, block_start, block_end, old, &new) {
            changed = true;
            break;
        }
        i = block_end;
    }

    if !changed {
        return Err(VdriftError::Adapter(format!(
            "no matching package entry in {}",
            path.display()
        )));
    }

    let rendered = lines.join("\n") + "\n";
    std::fs::write(path, rendered)
        .map_err(|e| VdriftError::Adapter(format!("cannot write {}: {e}", path.display())))
}

fn sibling_crate_name(lock: &std::path::Path) -> Result<Option<String>> {
    let manifest = lock
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("Cargo.toml");
    if !manifest.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&manifest)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", manifest.display())))?;
    let value: toml::Value = toml::from_str(&text).map_err(|e| {
        VdriftError::Adapter(format!("invalid TOML in {}: {e}", manifest.display()))
    })?;
    Ok(value
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(String::from))
}
