use crate::errors::{Result, VdriftError};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionCfg {
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReferencesCfg {
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorCfg {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub auto_bump: Option<bool>,
    #[serde(default)]
    pub auto_commit: Option<bool>,
}

/// Optional per-repository configuration: `.vdrift.toml`.
///
/// ```toml
/// [version]
/// source = "package.json"
///
/// [references]
/// files = ["src/version.ts", "README.md"]
///
/// [behavior]
/// enabled = true
/// auto_bump = true
/// auto_commit = true
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub version: VersionCfg,
    #[serde(default)]
    pub references: ReferencesCfg,
    #[serde(default)]
    pub behavior: BehaviorCfg,
}

impl ProjectConfig {
    pub fn load(root: &Path) -> Result<ProjectConfig> {
        let path = root.join(".vdrift.toml");
        if !path.is_file() {
            return Ok(ProjectConfig::default());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| VdriftError::Config(format!("cannot read {}: {e}", path.display())))?;
        let config: ProjectConfig = toml::from_str(&text)
            .map_err(|e| VdriftError::Config(format!("invalid .vdrift.toml: {e}")))?;
        Ok(config)
    }

    pub fn version_source(&self) -> Option<String> {
        self.version.source.clone()
    }

    pub fn reference_files(&self) -> &[String] {
        &self.references.files
    }

    pub fn enabled(&self) -> bool {
        self.behavior.enabled.unwrap_or(true)
    }

    pub fn auto_bump(&self) -> bool {
        self.behavior.auto_bump.unwrap_or(false)
    }

    pub fn auto_commit(&self) -> bool {
        self.behavior.auto_commit.unwrap_or(false)
    }
}

/// Reads the package name out of a Cargo.toml (used for project detection).
pub fn read_toml_package_name(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(String::from)
}
