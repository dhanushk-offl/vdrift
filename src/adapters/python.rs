use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;
use std::path::Path;

/// Python packaging adapter: `pyproject.toml` (PEP 621 / Poetry), `setup.py`,
/// `setup.cfg`, and the conventional `__version__` module attribute.
pub struct PythonAdapter;

const VERSION_KEYS: &[&str] = &["version =", "version="];
const DUNDER_KEYS: &[&str] = &["__version__ =", "__version__="];

impl super::VersionAdapter for PythonAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();

        let pyproject = repo.root.join("pyproject.toml");
        if pyproject.is_file() {
            // Static PEP 621 `[project] version`, else Poetry `[tool.poetry] version`.
            let current = util::read_toml_keys(&pyproject, &["project", "version"])?.or(
                util::read_toml_keys(&pyproject, &["tool", "poetry", "version"])?,
            );
            refs.push(VersionReference::new(
                pyproject.clone(),
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        let setup_py = repo.root.join("setup.py");
        if setup_py.is_file() {
            let current = util::read_line_version(&setup_py, VERSION_KEYS)?;
            refs.push(VersionReference::new(
                setup_py,
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        let setup_cfg = repo.root.join("setup.cfg");
        if setup_cfg.is_file() {
            let current = util::read_section_keys(&setup_cfg, "metadata", VERSION_KEYS)?;
            refs.push(VersionReference::new(
                setup_cfg,
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        // Conventional generated version modules.
        let name = pyproject_name(&pyproject);
        if let Some(pkg) = name {
            for rel in [
                format!("{pkg}/_version.py"),
                format!("src/{pkg}/_version.py"),
                format!("{pkg}/__init__.py"),
                format!("src/{pkg}/__init__.py"),
            ] {
                let path = repo.root.join(&rel);
                if !path.is_file() {
                    continue;
                }
                if let Some(current) = util::read_line_version(&path, DUNDER_KEYS)? {
                    refs.push(VersionReference::new(
                        path,
                        Some(current),
                        ReferenceKind::Reference,
                        true,
                    ));
                    break;
                }
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
            "pyproject.toml" => {
                // Locate the section that actually carries the version.
                if util::write_section_keys(&reference.file, "project", VERSION_KEYS, old, version)?
                {
                    return Ok(());
                }
                if util::write_section_keys(
                    &reference.file,
                    "tool.poetry",
                    VERSION_KEYS,
                    old,
                    version,
                )? {
                    return Ok(());
                }
                Err(VdriftError::Adapter(format!(
                    "no static version in {}",
                    reference.file.display()
                )))
            }
            "setup.py" => {
                if !util::write_line_version(&reference.file, VERSION_KEYS, old, version)? {
                    return Err(VdriftError::Adapter(format!(
                        "no version assignment in {}",
                        reference.file.display()
                    )));
                }
                Ok(())
            }
            "setup.cfg" => {
                if !util::write_section_keys(
                    &reference.file,
                    "metadata",
                    VERSION_KEYS,
                    old,
                    version,
                )? {
                    return Err(VdriftError::Adapter(format!(
                        "no version in [metadata] of {}",
                        reference.file.display()
                    )));
                }
                Ok(())
            }
            "_version.py" | "__init__.py" => {
                if !util::write_line_version(&reference.file, DUNDER_KEYS, old, version)? {
                    return Err(VdriftError::Adapter(format!(
                        "no __version__ in {}",
                        reference.file.display()
                    )));
                }
                Ok(())
            }
            _ => Err(VdriftError::Adapter(format!(
                "python adapter cannot update {}",
                reference.file.display()
            ))),
        }
    }
}

fn pyproject_name(pyproject: &Path) -> Option<String> {
    let text = std::fs::read_to_string(pyproject).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            value
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
}
