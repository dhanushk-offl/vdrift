use crate::core::detection::DetectionResult;
use crate::core::project::Project;
use crate::core::synchronization::Change;
use crate::core::version::Version;
use serde::Serialize;
use std::path::Path;

pub fn error_body(code: &str, message: &str) -> serde_json::Value {
    serde_json::json!({ "error": { "code": code, "message": message } })
}

#[derive(Debug, Serialize)]
pub struct ReasonOut {
    #[serde(rename = "type")]
    pub r#type: &'static str,
    pub confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct ChangeOut {
    pub file: String,
    pub from: Option<String>,
    pub to: String,
}

impl ChangeOut {
    pub fn from_change(change: &Change, root: &Path) -> ChangeOut {
        ChangeOut {
            file: change
                .file
                .strip_prefix(root)
                .unwrap_or(&change.file)
                .display()
                .to_string(),
            from: change.from.as_ref().map(|v| v.to_string()),
            to: change.to.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlanOutput {
    pub status: String,
    pub project: Project,
    pub current_version: Option<String>,
    pub suggested_version: Option<String>,
    pub reason: ReasonOut,
    pub changes: Vec<ChangeOut>,
}

#[derive(Debug, Serialize)]
pub struct VersionOut {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyOutput {
    pub status: String,
    pub version: VersionOut,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DriftOut {
    pub file: String,
    pub found: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerifyOutput {
    pub status: String,
    pub expected: Option<String>,
    pub drifts: Vec<DriftOut>,
}

#[derive(Debug, Serialize)]
pub struct CanonicalOut {
    pub file: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct OutdatedOut {
    pub file: String,
    pub version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckOutput {
    pub status: String,
    pub canonical: Option<CanonicalOut>,
    pub outdated: Vec<OutdatedOut>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SourceOut {
    pub file: String,
    pub version: Option<String>,
    pub kind: &'static str,
    pub writable: bool,
}

#[derive(Debug, Serialize)]
pub struct ScanOutput {
    pub project: Project,
    pub sources: Vec<SourceOut>,
    pub references: Vec<SourceOut>,
    pub candidates: Vec<SourceOut>,
    pub status: String,
}

impl ScanOutput {
    pub fn from_detection(
        detection: &DetectionResult,
        candidates: Vec<SourceOut>,
        root: &Path,
        canonical_version: Option<&Version>,
    ) -> ScanOutput {
        let mut sources = Vec::new();
        let mut references = Vec::new();
        for r in &detection.refs {
            let out = SourceOut {
                file: r
                    .file
                    .strip_prefix(root)
                    .unwrap_or(&r.file)
                    .display()
                    .to_string(),
                version: r.current.as_ref().map(|v| v.to_string()),
                kind: r.kind.label(),
                writable: r.writable,
            };
            if r.kind == crate::core::detection::ReferenceKind::Canonical
                || r.kind == crate::core::detection::ReferenceKind::Derived
            {
                sources.push(out);
            } else {
                references.push(out);
            }
        }
        let status = match canonical_version {
            Some(_) if detection.drifted_refs().is_empty() => "synchronized",
            Some(_) => "drift",
            None => "no_project",
        };
        ScanOutput {
            project: detection.project.clone(),
            sources,
            references,
            candidates,
            status: status.to_string(),
        }
    }
}
