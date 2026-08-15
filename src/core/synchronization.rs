use crate::adapters;
use crate::config::project::ProjectConfig;
use crate::core::detection::{DetectionResult, VersionReference};
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;

/// A file change that will be (or was) applied.
#[derive(Debug, Clone)]
pub struct Change {
    pub file: std::path::PathBuf,
    pub from: Option<Version>,
    pub to: Version,
}

/// Read-only list of every writable reference that would change if the target
/// version were applied.
pub fn plan_changes(detection: &DetectionResult, target: &Version) -> Vec<Change> {
    let mut changes = Vec::new();
    for r in &detection.refs {
        if !r.writable {
            continue;
        }
        let Some(current) = r.current.clone() else {
            continue;
        };
        if &current == target {
            continue;
        }
        changes.push(Change {
            file: r.file.clone(),
            from: Some(current),
            to: target.clone(),
        });
    }
    changes
}

/// Applies the canonical version to every writable reference.
///
/// Safety: refuses to modify files that carry uncommitted changes unless
/// `force` is set. Returns the list of changes actually applied.
pub fn sync_references(
    repo: &Repository,
    refs: &[VersionReference],
    target: &Version,
    config: &ProjectConfig,
    dry_run: bool,
    force: bool,
) -> Result<Vec<Change>> {
    // Determine which references would change, and pre-flight the dirty-tree
    // guard for all of them before writing anything (atomic safety check).
    let mut pending: Vec<&VersionReference> = Vec::new();
    for reference in refs {
        if !reference.writable {
            continue;
        }
        let Some(current) = &reference.current else {
            // Auto-detected references without a readable version (e.g. a
            // workspace manifest with no version field) are simply skipped.
            // Explicitly configured references must resolve or the user is
            // told what to fix.
            if reference.kind == crate::core::detection::ReferenceKind::Reference {
                return Err(VdriftError::Adapter(format!(
                    "cannot determine the current version in {} — add a recognizable version or remove it from [references]",
                    reference.display_path(&repo.root)
                )));
            }
            continue;
        };
        if current == target {
            continue;
        }
        if !dry_run && !force && crate::git::repository::is_dirty(repo, &reference.file)? {
            return Err(VdriftError::UnsafeTree(format!(
                "cannot safely update {} — the file contains uncommitted changes.\n\
                 Use --force if you understand the risk.",
                reference.display_path(&repo.root)
            )));
        }
        pending.push(reference);
    }

    let mut changes = Vec::new();
    for reference in pending {
        let current = reference.current.as_ref().unwrap_or(target).clone();
        if !dry_run {
            let adapter = adapters::adapter_for(&reference.file, config);
            adapter.update(reference, target)?;
        }
        changes.push(Change {
            file: reference.file.clone(),
            from: Some(current),
            to: target.clone(),
        });
    }

    Ok(changes)
}
