use crate::core::detection::VersionReference;
use crate::core::version::Version;

/// Result of comparing every writable reference against the canonical version.
#[derive(Debug)]
pub struct Verification {
    pub drifts: Vec<(std::path::PathBuf, Option<Version>)>,
}

impl Verification {
    pub fn is_consistent(&self) -> bool {
        self.drifts.is_empty()
    }
}

pub fn verify(refs: &[VersionReference], canonical: Option<Version>) -> Verification {
    let mut drifts = Vec::new();
    if let Some(canon) = &canonical {
        for r in refs {
            if !r.writable {
                continue;
            }
            if r.current.as_ref() != Some(canon) {
                drifts.push((r.file.clone(), r.current.clone()));
            }
        }
    }
    Verification { drifts }
}
