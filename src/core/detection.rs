use crate::adapters;
use crate::config::project::ProjectConfig;
use crate::core::project::Project;
use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use crate::git::repository::Repository;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Classification of a version reference within a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReferenceKind {
    /// Source of truth for the project version.
    Canonical,
    /// Generated from the canonical source (e.g. lockfiles).
    Derived,
    /// Explicitly configured or known reference that should match the canonical version.
    Reference,
    /// Detected by heuristics; never modified without explicit configuration.
    Candidate,
}

impl ReferenceKind {
    pub fn label(&self) -> &'static str {
        match self {
            ReferenceKind::Canonical => "canonical",
            ReferenceKind::Derived => "derived",
            ReferenceKind::Reference => "reference",
            ReferenceKind::Candidate => "candidate",
        }
    }
}

/// A single version-bearing location in the repository.
#[derive(Debug, Clone, Serialize)]
pub struct VersionReference {
    pub file: PathBuf,
    pub current: Option<Version>,
    pub kind: ReferenceKind,
    pub writable: bool,
}

impl VersionReference {
    pub fn new(
        file: PathBuf,
        current: Option<Version>,
        kind: ReferenceKind,
        writable: bool,
    ) -> Self {
        VersionReference {
            file,
            current,
            kind,
            writable,
        }
    }

    pub fn display_path(&self, root: &Path) -> String {
        self.file
            .strip_prefix(root)
            .unwrap_or(&self.file)
            .display()
            .to_string()
    }
}

/// Result of the full detection pass over a repository.
#[derive(Debug)]
pub struct DetectionResult {
    pub project: Project,
    pub refs: Vec<VersionReference>,
    pub canonical: Option<Version>,
    pub canonical_file: Option<PathBuf>,
}

impl DetectionResult {
    /// Writable references with a known current version that disagree with the
    /// canonical version.
    pub fn drifted_refs(&self) -> Vec<&VersionReference> {
        let Some(canon) = &self.canonical else {
            return Vec::new();
        };
        self.refs
            .iter()
            .filter(|r| r.writable && r.current.is_some() && r.current.as_ref() != Some(canon))
            .collect()
    }
}

/// Priority used when multiple manifests could be canonical and agree.
fn canonical_priority(file: &Path) -> u8 {
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match name {
        "package.json" => 4,
        "Cargo.toml" | "tauri.conf.json" => 3,
        "pyproject.toml" | "pom.xml" => 2,
        "go.mod" => 1,
        _ => 0,
    }
}

/// Runs layered discovery and classifies every reference.
///
/// Layer 1 — known manifests (canonical/derived via adapters)
/// Layer 2 — generated files (lockfiles, derived)
/// Layer 3 — known source patterns (configured reference files)
/// Layer 4 — user configuration (.vdrift.toml)
pub fn detect(repo: &Repository, config: &ProjectConfig) -> Result<DetectionResult> {
    let project = Project::detect(&repo.root);
    let mut refs: Vec<VersionReference> = Vec::new();

    for adapter in adapters::all() {
        refs.extend(adapter.detect(repo)?);
    }

    let generic = adapters::generic(config);
    refs.extend(generic.detect(repo)?);

    // Multiple adapters may report the same file; keep the first (most
    // specific) report.
    let mut seen: Vec<PathBuf> = Vec::new();
    refs.retain(|r| {
        if seen.contains(&r.file) {
            return false;
        }
        seen.push(r.file.clone());
        true
    });

    let (canonical, canonical_file) = resolve_canonical(&mut refs, repo, config)?;

    // Pass 2 — for text-style references without a self-contained version,
    // extract a version token, falling back to an exact canonical match.
    if let Some(canon) = &canonical {
        let canonical_path = canonical_file.clone();
        for r in refs.iter_mut() {
            if r.kind == ReferenceKind::Reference
                && r.current.is_none()
                && canonical_path.as_deref() != Some(r.file.as_path())
            {
                if let Some(v) = extract_version_token(&r.file) {
                    r.current = Some(v);
                } else if file_contains(&r.file, &canon.to_string()) {
                    r.current = Some(canon.clone());
                }
            }
        }
    }

    Ok(DetectionResult {
        project,
        refs,
        canonical,
        canonical_file,
    })
}

fn resolve_canonical(
    refs: &mut [VersionReference],
    repo: &Repository,
    config: &ProjectConfig,
) -> Result<(Option<Version>, Option<PathBuf>)> {
    // Explicit configuration wins.
    if let Some(source) = config.version_source() {
        let path = repo.root.join(&source);
        // Downgrade every auto-detected canonical to a plain reference.
        for r in refs.iter_mut() {
            if r.kind == ReferenceKind::Canonical {
                r.kind = ReferenceKind::Reference;
            }
        }
        if let Some(r) = refs.iter_mut().find(|r| r.file == path) {
            r.kind = ReferenceKind::Canonical;
            return Ok((r.current.clone(), Some(path)));
        }
        return Ok((None, None));
    }

    let mut canonicals: Vec<VersionReference> = refs
        .iter()
        .filter(|r| r.kind == ReferenceKind::Canonical && r.current.is_some())
        .cloned()
        .collect();

    if canonicals.is_empty() {
        return Ok((None, None));
    }

    canonicals.sort_by_key(|r| std::cmp::Reverse(canonical_priority(&r.file)));

    let first_version = canonicals[0].current.clone();
    for other in &canonicals[1..] {
        if other.current != first_version {
            return Err(VdriftError::multiple_version_sources());
        }
    }

    let chosen = canonicals.remove(0);
    // Downgrade agreeing extra canonicals to references.
    for r in refs.iter_mut() {
        if r.kind == ReferenceKind::Canonical && r.file != chosen.file {
            r.kind = ReferenceKind::Reference;
        }
    }

    Ok((chosen.current.clone(), Some(chosen.file)))
}

/// Reads a file and checks whether it contains an exact version string.
fn file_contains(path: &Path, version: &str) -> bool {
    use std::fs;
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.len() > 1024 * 1024 {
        return false;
    }
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    if bytes.contains(&0) {
        return false; // binary
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return false;
    };
    text.contains(version)
}

/// Extracts the first plausible semver token from a plain-text file.
fn extract_version_token(path: &Path) -> Option<Version> {
    use std::fs;
    let Ok(meta) = fs::metadata(path) else {
        return None;
    };
    if !meta.is_file() || meta.len() > 1024 * 1024 {
        return None;
    }
    let Ok(bytes) = fs::read(path) else {
        return None;
    };
    if bytes.contains(&0) {
        return None; // binary
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return None;
    };
    for word in text.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| {
            !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
        });
        if trimmed.is_empty() || trimmed == word {
            if let Ok(v) = Version::parse(word) {
                return Some(v);
            }
        } else if let Ok(v) = Version::parse(trimmed) {
            return Some(v);
        }
    }
    None
}

/// Layer 5 — generic candidate scanning. Never writable by default.
/// Used by `vdrift scan` only, to stay fast for `check`/`verify`.
pub fn scan_candidates(repo: &Repository, canonical: &Version) -> Vec<VersionReference> {
    let needle = canonical.to_string();
    let mut out = Vec::new();
    let mut walker = ignore::WalkBuilder::new(&repo.root);
    walker
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);
    walker.filter_entry(|e| {
        let name = e.file_name().to_str().unwrap_or_default();
        !matches!(
            name,
            "node_modules" | "target" | "vendor" | "dist" | "build" | "coverage" | ".git"
        )
    });
    let skip = repo.root.clone();
    for entry in walker.build() {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if entry.path() == skip {
            continue;
        }
        let Some(rel) = entry.path().strip_prefix(&repo.root).ok() else {
            continue;
        };
        // Skip our own config file so we don't self-report drift.
        if rel == Path::new(".vdrift.toml") {
            continue;
        }
        if file_contains(entry.path(), &needle) {
            out.push(VersionReference::new(
                entry.path().to_path_buf(),
                Some(canonical.clone()),
                ReferenceKind::Candidate,
                false,
            ));
        }
    }
    out
}
