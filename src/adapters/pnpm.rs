use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// pnpm lockfile adapter. The root package's version lives under
/// `importers."."` in lockfile v6+; older/newer formats may omit it, in
/// which case no reference is reported.
pub struct PnpmAdapter;

const IMPORTER_KEYS: &[&str] = &["."];

impl super::VersionAdapter for PnpmAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let lock = repo.root.join("pnpm-lock.yaml");
        if !lock.is_file() {
            return Ok(Vec::new());
        }
        for key in IMPORTER_KEYS {
            if let Some(current) = util::read_yaml_keys(&lock, &["importers", key, "version"])? {
                return Ok(vec![VersionReference::new(
                    lock,
                    Some(current),
                    ReferenceKind::Derived,
                    true,
                )]);
            }
        }
        Ok(Vec::new())
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        if reference.file.file_name().and_then(|n| n.to_str()) != Some("pnpm-lock.yaml") {
            return Err(VdriftError::Adapter(format!(
                "pnpm adapter cannot update {}",
                reference.file.display()
            )));
        }
        for key in IMPORTER_KEYS {
            if util::read_yaml_keys(&reference.file, &["importers", key, "version"])?.is_some() {
                util::write_yaml_keys(&reference.file, &["importers", key, "version"], version)?;
                return Ok(());
            }
        }
        Err(VdriftError::Adapter(format!(
            "no root package version in {}",
            reference.file.display()
        )))
    }
}
