use crate::adapters::util;
use crate::core::detection::{ReferenceKind, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// Java / JVM adapter: Maven `pom.xml` and Gradle (`build.gradle`,
/// `build.gradle.kts`, `gradle.properties`).
pub struct JavaAdapter;

const GRADLE_KEYS: &[&str] = &["version =", "version=", "version "];
const PROPERTIES_KEYS: &[&str] = &["version=", "version ="];

impl super::VersionAdapter for JavaAdapter {
    fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
        let mut refs = Vec::new();

        let pom = repo.root.join("pom.xml");
        if pom.is_file() {
            let current = util::read_pom_version(&pom)?;
            refs.push(VersionReference::new(
                pom,
                current,
                ReferenceKind::Canonical,
                true,
            ));
        }

        for name in ["build.gradle", "build.gradle.kts"] {
            let gradle = repo.root.join(name);
            if gradle.is_file() {
                let current = util::read_line_version(&gradle, GRADLE_KEYS)?;
                refs.push(VersionReference::new(
                    gradle,
                    current,
                    ReferenceKind::Canonical,
                    true,
                ));
                break;
            }
        }

        let props = repo.root.join("gradle.properties");
        if props.is_file() {
            let current = util::read_line_version(&props, PROPERTIES_KEYS)?;
            refs.push(VersionReference::new(
                props,
                current,
                ReferenceKind::Canonical,
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
        let old = reference.current.as_ref();
        match name {
            "pom.xml" => util::write_pom_version(&reference.file, version),
            "build.gradle" | "build.gradle.kts" => {
                if !util::write_line_version(&reference.file, GRADLE_KEYS, old, version)? {
                    return Err(VdriftError::Adapter(format!(
                        "no version assignment in {}",
                        reference.file.display()
                    )));
                }
                Ok(())
            }
            "gradle.properties" => {
                if !util::write_line_version(&reference.file, PROPERTIES_KEYS, old, version)? {
                    return Err(VdriftError::Adapter(format!(
                        "no version in {}",
                        reference.file.display()
                    )));
                }
                Ok(())
            }
            _ => Err(VdriftError::Adapter(format!(
                "java adapter cannot update {}",
                reference.file.display()
            ))),
        }
    }
}
