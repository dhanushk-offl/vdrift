use crate::core::detection::{DetectionResult, ReferenceKind};
use crate::core::synchronization::Change;
use crate::core::version::Version;
use std::path::Path;

pub fn title(text: &str) {
    println!("{text}");
    println!();
}

pub fn ok() {
    println!("✓ version consistent");
}

pub fn section(label: &str) {
    println!("{label}");
    println!();
}

pub fn item(label: &str, value: &str) {
    println!("  {label:<24}{value}");
}

pub fn detect_row(file: &str, version: &Option<Version>, kind: ReferenceKind) {
    let version = version
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".into());
    println!("  {file:<24}{version:<16}{}", kind.label());
}

pub fn change_row(change: &Change, root: &Path) {
    let file = change
        .file
        .strip_prefix(root)
        .unwrap_or(&change.file)
        .display()
        .to_string();
    let from = change
        .from
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".into());
    println!("  {file:<24}{from} → {}", change.to);
}

pub fn scan(
    detection: &DetectionResult,
    candidates: &[crate::core::detection::VersionReference],
    root: &Path,
) {
    title("Project");
    item("type", detection.project.project_type.label());
    item("package manager", detection.project.package_manager.label());

    section("Version sources");
    for r in &detection.refs {
        if r.kind == ReferenceKind::Canonical || r.kind == ReferenceKind::Derived {
            let path = r
                .file
                .strip_prefix(root)
                .unwrap_or(&r.file)
                .display()
                .to_string();
            detect_row(&path, &r.current, r.kind);
        }
    }

    section("References");
    for r in &detection.refs {
        if r.kind == ReferenceKind::Reference {
            let path = r
                .file
                .strip_prefix(root)
                .unwrap_or(&r.file)
                .display()
                .to_string();
            detect_row(&path, &r.current, r.kind);
        }
    }

    if !candidates.is_empty() {
        section("Candidates (not writable)");
        for r in candidates {
            let path = r
                .file
                .strip_prefix(root)
                .unwrap_or(&r.file)
                .display()
                .to_string();
            detect_row(&path, &r.current, r.kind);
        }
    }

    section("Status");
    match &detection.canonical {
        Some(_) if detection.drifted_refs().is_empty() => println!("  ✓ synchronized"),
        Some(_) => println!("  ✗ drift detected"),
        None => println!("  no version detected"),
    }
}

pub fn drift_report(detection: &DetectionResult, root: &Path) {
    println!("version drift detected");
    println!();
    if let Some((file, version)) = detection
        .canonical_file
        .as_ref()
        .zip(detection.canonical.as_ref())
    {
        section("Canonical");
        let path = file
            .strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string();
        item(&path, &version.to_string());
    }
    let drifted = detection.drifted_refs();
    if !drifted.is_empty() {
        section("Outdated");
        for r in &drifted {
            let path = r
                .file
                .strip_prefix(root)
                .unwrap_or(&r.file)
                .display()
                .to_string();
            let v = r
                .current
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into());
            item(&path, &v);
        }
        println!();
        println!("{} reference(s) need updating.", drifted.len());
    }
}
