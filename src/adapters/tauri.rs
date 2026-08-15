use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// Tauri app versioning: `tauri.conf.json` drives the released app version,
/// and `src-tauri/Cargo.toml` must stay in step with it.
pub struct TauriAdapter;

const TAURI_CONFIGS: &[&str] = &["tauri.conf.json", "src-tauri/tauri.conf.json"];

impl super::VersionAdapter for TauriAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();

        for rel in TAURI_CONFIGS {
            let path = repo.root.join(rel);
            if path.is_file() {
                let current = util::read_json_version(&path)?;
                refs.push(VersionReference::new(
                    path,
                    current,
                    ReferenceKind::Canonical,
                    true,
                ));
                break;
            }
        }

        let manifest = repo.root.join("src-tauri/Cargo.toml");
        if manifest.is_file() {
            let current = util::read_toml_keys(&manifest, &["package", "version"])?;
            refs.push(VersionReference::new(
                manifest,
                current,
                ReferenceKind::Reference,
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
            "tauri.conf.json" => util::write_json_version(&reference.file, version),
            "Cargo.toml" => {
                let old = reference.current.as_ref();
                if !util::write_section_keys(
                    &reference.file,
                    "package",
                    &["version =", "version="],
                    old,
                    version,
                )? {
                    return Err(VdriftError::Adapter(format!(
                        "no version in [package] of {}",
                        reference.file.display()
                    )));
                }
                Ok(())
            }
            _ => Err(VdriftError::Adapter(format!(
                "tauri adapter cannot update {}",
                reference.file.display()
            ))),
        }
    }
}
