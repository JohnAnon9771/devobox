use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::Service;

/// Represents a logical project workspace (NOT a container)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Project name (directory name)
    pub name: String,
    /// Absolute path to project directory
    pub path: PathBuf,
    /// Associated configuration
    pub config: ProjectConfig,
}

/// Configuration loaded from project's devobox.toml
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectConfig {
    /// Project-specific settings
    #[serde(default)]
    pub project: Option<ProjectSettings>,

    /// Dependencies on other projects (for service resolution)
    #[serde(default)]
    pub dependencies: ProjectDependencies,

    /// Project-specific services
    #[serde(default)]
    pub services: Option<HashMap<String, Service>>,
}

/// Project-specific settings
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectSettings {
    /// Explicit project name (overrides directory name)
    #[serde(default)]
    pub name: Option<String>,

    /// Environment variables to set
    #[serde(default)]
    pub env: Vec<String>,

    /// Shell to use (bash, zsh, fish)
    #[serde(default)]
    pub shell: Option<String>,

    /// Command to run when starting the project
    #[serde(default)]
    pub startup_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectDependencies {
    /// Other projects to include services from
    #[serde(default)]
    pub include_projects: Vec<PathBuf>,
}

impl Project {
    /// Creates a new Project instance
    /// Resolves the name automatically: Config > Directory Name > "unknown"
    pub fn new(path: PathBuf, config: ProjectConfig) -> Self {
        let name = config
            .project
            .as_ref()
            .and_then(|p| p.name.clone())
            .or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        Self { name, path, config }
    }

    /// Zellij session name for this project
    pub fn session_name(&self) -> String {
        format!("devobox-{}", self.name)
    }

    /// Get environment variables for this project
    pub fn env_vars(&self) -> &[String] {
        self.config
            .project
            .as_ref()
            .map(|p| p.env.as_slice())
            .unwrap_or(&[])
    }

    /// Get preferred shell for this project
    pub fn shell(&self) -> Option<&str> {
        self.config
            .project
            .as_ref()
            .and_then(|p| p.shell.as_deref())
    }

    /// Get startup command for this project
    pub fn startup_command(&self) -> Option<&str> {
        self.config
            .project
            .as_ref()
            .and_then(|p| p.startup_command.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_generation() {
        let project = Project::new(
            PathBuf::from("/home/dev/code/my-app"),
            ProjectConfig::default(),
        );
        assert_eq!(project.name, "my-app");
        assert_eq!(project.session_name(), "devobox-my-app");
    }

    #[test]
    fn test_project_name_from_config_override() {
        let config = ProjectConfig {
            project: Some(ProjectSettings {
                name: Some("custom-name".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let project = Project::new(PathBuf::from("/home/dev/code/my-app"), config);
        assert_eq!(project.name, "custom-name");
        assert_eq!(project.session_name(), "devobox-custom-name");
    }

    #[test]
    fn test_project_config_defaults() {
        let config = ProjectConfig::default();
        assert!(config.project.is_none());
        assert!(config.dependencies.include_projects.is_empty());
    }

    #[test]
    fn test_env_vars_empty_when_no_config() {
        let project = Project::new(PathBuf::from("/test"), ProjectConfig::default());
        assert!(project.env_vars().is_empty());
    }

    #[test]
    fn test_env_vars_from_config() {
        let config = ProjectConfig {
            project: Some(ProjectSettings {
                env: vec!["NODE_ENV=development".into(), "DEBUG=app:*".into()],
                shell: None,
                startup_command: None,
                name: None,
            }),
            ..Default::default()
        };

        let project = Project::new(PathBuf::from("/test"), config);
        assert_eq!(project.env_vars().len(), 2);
        assert_eq!(project.env_vars()[0], "NODE_ENV=development");
    }
}
