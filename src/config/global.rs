use crate::errors::{Result, VdriftError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Machine-level configuration stored in the OS config directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// vdrift version that last ran `init`.
    pub installed_version: Option<String>,
    /// Directory Git is configured to load hooks from (managed by vdrift).
    pub hooks_path: Option<String>,
    /// The `core.hooksPath` value present before vdrift took over, if any.
    pub previous_hooks_path: Option<String>,
    /// Whether global integration is enabled.
    pub enabled: bool,
}

/// OS-specific config directory: `~/.config/vdrift`, `~/Library/Application Support/vdrift`, `%APPDATA%\vdrift`.
pub fn global_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| {
        VdriftError::Config("could not determine the OS configuration directory".to_string())
    })?;
    Ok(base.join("vdrift"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(global_dir()?.join("config.toml"))
}

pub fn hooks_dir() -> Result<PathBuf> {
    Ok(global_dir()?.join("hooks"))
}

impl GlobalConfig {
    pub fn load() -> Result<GlobalConfig> {
        let path = config_path()?;
        if !path.is_file() {
            return Ok(GlobalConfig::default());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| VdriftError::Config(format!("cannot read {}: {e}", path.display())))?;
        toml::from_str(&text).map_err(|e| {
            VdriftError::Config(format!("invalid global config {}: {e}", path.display()))
        })
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VdriftError::Config(format!("cannot create {}: {e}", parent.display()))
            })?;
        }
        let rendered = toml::to_string(self)
            .map_err(|e| VdriftError::Config(format!("failed to serialize global config: {e}")))?;
        std::fs::write(&path, rendered)
            .map_err(|e| VdriftError::Config(format!("cannot write {}: {e}", path.display())))
    }

    pub fn is_installed(&self) -> bool {
        self.enabled && self.hooks_path.is_some()
    }
}
