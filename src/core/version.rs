use crate::errors::{Result, VdriftError};
use semver::Version as SemVersion;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// SemVer bump level used by the proposal engine and `vdrift bump`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BumpLevel {
    None,
    Patch,
    Minor,
    Major,
}

/// A validated semantic version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub SemVersion);

impl Version {
    pub fn parse(input: &str) -> Result<Version> {
        let input = input.trim();
        SemVersion::parse(input)
            .map(Version)
            .map_err(|e| VdriftError::InvalidVersion(format!("invalid version `{input}`: {e}")))
    }

    pub fn bump(&self, level: BumpLevel) -> Version {
        match level {
            BumpLevel::None => self.clone(),
            BumpLevel::Patch => {
                let mut v = self.0.clone();
                v.patch += 1;
                v.pre = semver::Prerelease::EMPTY;
                v.build = semver::BuildMetadata::EMPTY;
                Version(v)
            }
            BumpLevel::Minor => {
                let mut v = self.0.clone();
                v.minor += 1;
                v.patch = 0;
                v.pre = semver::Prerelease::EMPTY;
                v.build = semver::BuildMetadata::EMPTY;
                Version(v)
            }
            BumpLevel::Major => {
                let mut v = self.0.clone();
                v.major += 1;
                v.minor = 0;
                v.patch = 0;
                v.pre = semver::Prerelease::EMPTY;
                v.build = semver::BuildMetadata::EMPTY;
                Version(v)
            }
        }
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Version {
    type Err = VdriftError;

    fn from_str(s: &str) -> Result<Version> {
        Version::parse(s)
    }
}
