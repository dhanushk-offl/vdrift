use serde::{Deserialize, Serialize};

/// Detected project ecosystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Node,
    Cargo,
    Unknown,
}

impl ProjectType {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectType::Node => "Node.js",
            ProjectType::Cargo => "Rust",
            ProjectType::Unknown => "Unknown",
        }
    }
}

/// Package manager used to drive the project, when known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Cargo,
    None,
}

impl PackageManager {
    pub fn label(&self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Cargo => "Cargo",
            PackageManager::None => "unknown",
        }
    }
}

/// Identifies the project type and package manager from a repository root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    #[serde(rename = "type")]
    pub project_type: ProjectType,
    pub package_manager: PackageManager,
    /// Name of the root package/manifest, if known.
    pub name: Option<String>,
}

impl Project {
    pub fn detect(root: &std::path::Path) -> Project {
        let manifest = root.join("package.json");
        if manifest.exists() {
            let name = serde_json::from_str::<serde_json::Value>(
                &std::fs::read_to_string(&manifest).unwrap_or_default(),
            )
            .ok()
            .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(String::from));
            return Project {
                project_type: ProjectType::Node,
                package_manager: PackageManager::Npm,
                name,
            };
        }

        if root.join("Cargo.toml").exists() {
            let name = crate::config::project::read_toml_package_name(&root.join("Cargo.toml"));
            return Project {
                project_type: ProjectType::Cargo,
                package_manager: PackageManager::Cargo,
                name,
            };
        }

        Project {
            project_type: ProjectType::Unknown,
            package_manager: PackageManager::None,
            name: None,
        }
    }
}
