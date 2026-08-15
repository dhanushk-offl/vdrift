use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// Go module versioning is tag-based, so `go.mod` carries no module version.
/// This adapter reads/writes the conventional embedded version file:
///
/// ```go
/// var Version = "1.2.3"
/// ```
///
/// at a fixed set of well-known locations.
pub struct GoAdapter;

const GO_VERSION_FILES: &[&str] = &[
    "version.go",
    "internal/version/version.go",
    "pkg/version/version.go",
    "src/version/version.go",
];

const GO_VERSION_KEYS: &[&str] = &[
    "var Version = ",
    "const Version = ",
    "var VERSION = ",
    "const VERSION = ",
    "var version = ",
    "var Version=",
    "const Version=",
];

impl super::VersionAdapter for GoAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();
        for rel in GO_VERSION_FILES {
            let path = repo.root.join(rel);
            if !path.is_file() {
                continue;
            }
            if let Some(current) = util::read_line_version(&path, GO_VERSION_KEYS)? {
                refs.push(VersionReference::new(
                    path,
                    Some(current),
                    ReferenceKind::Canonical,
                    true,
                ));
            }
        }
        Ok(refs)
    }

    fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
        if reference.file.file_name().and_then(|n| n.to_str()) != Some("version.go") {
            return Err(VdriftError::Adapter(format!(
                "go adapter cannot update {}",
                reference.file.display()
            )));
        }
        let old = reference.current.as_ref();
        if !util::write_line_version(&reference.file, GO_VERSION_KEYS, old, version)? {
            return Err(VdriftError::Adapter(format!(
                "no version assignment in {}",
                reference.file.display()
            )));
        }
        Ok(())
    }
}
