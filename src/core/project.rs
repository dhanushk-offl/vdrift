use serde::{Deserialize, Serialize};

/// Detected project ecosystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Node,
    Cargo,
    Tauri,
    Python,
    Go,
    Java,
    Dart,
    Elixir,
    Php,
    Ruby,
    Haskell,
    Unknown,
}

impl ProjectType {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectType::Node => "Node.js",
            ProjectType::Cargo => "Rust",
            ProjectType::Tauri => "Tauri",
            ProjectType::Python => "Python",
            ProjectType::Go => "Go",
            ProjectType::Java => "Java",
            ProjectType::Dart => "Dart/Flutter",
            ProjectType::Elixir => "Elixir",
            ProjectType::Php => "PHP",
            ProjectType::Ruby => "Ruby",
            ProjectType::Haskell => "Haskell",
            ProjectType::Unknown => "Unknown",
        }
    }
}

/// Package manager used to drive the project, when known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Pnpm,
    Bun,
    Yarn,
    Cargo,
    Pip,
    Poetry,
    GoMod,
    Maven,
    Gradle,
    Pub,
    Mix,
    Composer,
    Bundler,
    Cabal,
    None,
}

impl PackageManager {
    pub fn label(&self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Bun => "bun",
            PackageManager::Yarn => "yarn",
            PackageManager::Cargo => "Cargo",
            PackageManager::Pip => "pip",
            PackageManager::Poetry => "Poetry",
            PackageManager::GoMod => "go modules",
            PackageManager::Maven => "Maven",
            PackageManager::Gradle => "Gradle",
            PackageManager::Pub => "pub",
            PackageManager::Mix => "Mix",
            PackageManager::Composer => "Composer",
            PackageManager::Bundler => "Bundler",
            PackageManager::Cabal => "Cabal",
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
        if root.join("package.json").exists() {
            let manager = if root.join("pnpm-lock.yaml").exists() {
                PackageManager::Pnpm
            } else if root.join("bun.lock").exists() {
                PackageManager::Bun
            } else if root.join("yarn.lock").exists() {
                PackageManager::Yarn
            } else {
                PackageManager::Npm
            };
            return Project {
                project_type: ProjectType::Node,
                package_manager: manager,
                name: json_name(&root.join("package.json")),
            };
        }

        if root.join("tauri.conf.json").exists() || root.join("src-tauri/tauri.conf.json").exists()
        {
            return Project {
                project_type: ProjectType::Tauri,
                package_manager: PackageManager::Cargo,
                name: None,
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

        if root.join("pyproject.toml").exists() {
            let manager = if has_toml_table(&root.join("pyproject.toml"), "tool.poetry") {
                PackageManager::Poetry
            } else {
                PackageManager::Pip
            };
            return Project {
                project_type: ProjectType::Python,
                package_manager: manager,
                name: crate::config::project::read_toml_package_name(&root.join("pyproject.toml")),
            };
        }

        if root.join("setup.py").exists() || root.join("setup.cfg").exists() {
            return Project {
                project_type: ProjectType::Python,
                package_manager: PackageManager::Pip,
                name: None,
            };
        }

        if root.join("pom.xml").exists() {
            return Project {
                project_type: ProjectType::Java,
                package_manager: PackageManager::Maven,
                name: None,
            };
        }

        if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
            return Project {
                project_type: ProjectType::Java,
                package_manager: PackageManager::Gradle,
                name: None,
            };
        }

        if root.join("go.mod").exists() || root.join("version.go").is_file() {
            let mod_name = std::fs::read_to_string(root.join("go.mod"))
                .ok()
                .and_then(|t| {
                    t.lines().find_map(|l| {
                        let l = l.trim();
                        l.strip_prefix("module ").map(|m| m.trim().to_string())
                    })
                });
            return Project {
                project_type: ProjectType::Go,
                package_manager: PackageManager::GoMod,
                name: mod_name,
            };
        }

        if root.join("pubspec.yaml").exists() {
            return Project {
                project_type: ProjectType::Dart,
                package_manager: PackageManager::Pub,
                name: None,
            };
        }

        if root.join("mix.exs").exists() {
            return Project {
                project_type: ProjectType::Elixir,
                package_manager: PackageManager::Mix,
                name: None,
            };
        }

        if root.join("composer.json").exists() {
            return Project {
                project_type: ProjectType::Php,
                package_manager: PackageManager::Composer,
                name: json_name(&root.join("composer.json")),
            };
        }

        if let Some(gemspec) = find_gemspec(root) {
            return Project {
                project_type: ProjectType::Ruby,
                package_manager: PackageManager::Bundler,
                name: Some(gemspec),
            };
        }

        let has_cabal = root
            .read_dir()
            .ok()
            .map(|d| {
                d.filter_map(|e| e.ok()).any(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.ends_with(".cabal")
                })
            })
            .unwrap_or(false);
        if root.join("package.yaml").exists() || has_cabal {
            return Project {
                project_type: ProjectType::Haskell,
                package_manager: PackageManager::Cabal,
                name: None,
            };
        }

        Project {
            project_type: ProjectType::Unknown,
            package_manager: PackageManager::None,
            name: None,
        }
    }
}

fn json_name(path: &std::path::Path) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(path).ok()?)
        .ok()
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
}

/// True when the TOML document contains the given (possibly dotted) table.
fn has_toml_table(path: &std::path::Path, table: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return false;
    };
    value.get(table).map(|v| v.is_table()).unwrap_or(false)
}

/// Name of a `*.gemspec` file at the repo root, without the extension.
fn find_gemspec(root: &std::path::Path) -> Option<String> {
    let entries = root.read_dir().ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".gemspec") {
            return Some(name.trim_end_matches(".gemspec").to_string());
        }
    }
    None
}
