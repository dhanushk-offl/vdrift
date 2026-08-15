use crate::core::version::BumpLevel;
use serde::Serialize;

/// Reason behind a suggested version change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasonType {
    Major,
    Feature,
    Fix,
    None,
}

/// Deterministic, Conventional-Commits-based version proposal.
#[derive(Debug, Clone, Serialize)]
pub struct Proposal {
    pub level: BumpLevel,
    #[serde(rename = "type")]
    pub reason: ReasonType,
    pub confidence: f64,
}

impl Proposal {
    pub fn none() -> Self {
        Proposal {
            level: BumpLevel::None,
            reason: ReasonType::None,
            confidence: 0.0,
        }
    }

    pub fn from_level(level: BumpLevel, source_count: usize) -> Self {
        let (reason, confidence) = match level {
            BumpLevel::Major => (ReasonType::Major, 0.98),
            BumpLevel::Minor => (ReasonType::Feature, 0.92),
            BumpLevel::Patch => (ReasonType::Fix, 0.85),
            BumpLevel::None => (ReasonType::None, 0.0),
        };
        let confidence = (confidence + (source_count as f64 * 0.01)).min(0.999);
        Proposal {
            level,
            reason,
            confidence,
        }
    }

    pub fn needs_update(&self) -> bool {
        self.level != BumpLevel::None
    }
}

fn classify_prefix(msg: &str) -> BumpLevel {
    let lower = msg.to_lowercase();
    if lower.contains("breaking change") || lower.contains("breaking-change") {
        return BumpLevel::Major;
    }
    // Skip scope-less prefixes: the first word up to ':' or '(' or ' '.
    let word: String = lower
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    let after: Option<char> = lower.chars().nth(word.len());
    let bang = after == Some('!');

    let level = match word.as_str() {
        "feat" => BumpLevel::Minor,
        "fix" | "perf" | "refactor" => BumpLevel::Patch,
        "docs" | "chore" | "test" | "build" | "ci" | "style" => BumpLevel::None,
        _ => {
            // Non-conventional message. Heuristic: treat "add"/"new"/"support" as minor-ish? Keep conservative: none.
            return BumpLevel::None;
        }
    };
    if bang && level != BumpLevel::None {
        return BumpLevel::Major;
    }
    level
}

/// Analyzes a batch of commit messages and proposes a bump.
pub fn analyze(messages: &[String]) -> Proposal {
    if messages.is_empty() {
        return Proposal::none();
    }
    let mut level = BumpLevel::None;
    let mut contributors = 0usize;
    for msg in messages {
        let l = classify_prefix(msg);
        match (l, level) {
            (BumpLevel::None, _) => {}
            (BumpLevel::Major, _) => {
                level = BumpLevel::Major;
                contributors += 1;
            }
            (BumpLevel::Minor, BumpLevel::Major) => {}
            (BumpLevel::Minor, _) => {
                level = BumpLevel::Minor;
                contributors += 1;
            }
            (BumpLevel::Patch, BumpLevel::None) => {
                level = BumpLevel::Patch;
                contributors += 1;
            }
            (BumpLevel::Patch, _) => {
                contributors += 1;
            }
        }
    }
    Proposal::from_level(level, contributors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_feat_as_minor() {
        let p = analyze(&["feat: add billing".into()]);
        assert_eq!(p.level, BumpLevel::Minor);
        assert_eq!(p.reason, ReasonType::Feature);
    }

    #[test]
    fn classifies_fix_as_patch() {
        let p = analyze(&["fix: crash on login".into()]);
        assert_eq!(p.level, BumpLevel::Patch);
    }

    #[test]
    fn breaking_change_is_major() {
        let p = analyze(&["feat: new api\n\nBREAKING CHANGE: api changed".into()]);
        assert_eq!(p.level, BumpLevel::Major);
    }

    #[test]
    fn docs_is_none() {
        let p = analyze(&["docs: update readme".into()]);
        assert_eq!(p.level, BumpLevel::None);
    }

    #[test]
    fn empty_is_none() {
        assert_eq!(analyze(&[]).level, BumpLevel::None);
    }

    #[test]
    fn bang_is_major() {
        let p = analyze(&["feat!: drop old endpoint".into()]);
        assert_eq!(p.level, BumpLevel::Major);
    }
}
