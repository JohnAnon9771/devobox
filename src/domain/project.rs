use serde::Deserialize;
use std::path::PathBuf;

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
}

/// Project-specific settings
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectSettings {
    /// Environment variables to set
    #[serde(default)]
    pub env: Vec<String>,

    /// Shell to use (bash, zsh, fish)
    #[serde(default)]
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectDependencies {
    /// Path to services.yml file (relative to project directory)
    #[serde(default)]
    pub services_yml: Option<PathBuf>,

    /// Other projects to include services from
    #[serde(default)]
    pub include_projects: Vec<PathBuf>,
}

impl Project {
    pub fn new(name: String, path: PathBuf, config: ProjectConfig) -> Self {
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

    /// Get services.yml path if configured
    pub fn services_yml_path(&self) -> Option<PathBuf> {
        self.config
            .dependencies
            .services_yml
            .as_ref()
            .map(|p| self.path.join(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_generation() {
        let project = Project::new(
            "my-app".into(),
            PathBuf::from("/home/dev/code/my-app"),
            ProjectConfig::default(),
        );
        assert_eq!(project.session_name(), "devobox-my-app");
    }

    #[test]
    fn test_project_config_defaults() {
        let config = ProjectConfig::default();
        assert!(config.project.is_none());
        assert!(config.dependencies.services_yml.is_none());
        assert!(config.dependencies.include_projects.is_empty());
    }

    #[test]
    fn test_env_vars_empty_when_no_config() {
        let project = Project::new(
            "test".into(),
            PathBuf::from("/test"),
            ProjectConfig::default(),
        );
        assert!(project.env_vars().is_empty());
    }

    #[test]
    fn test_env_vars_from_config() {
        let config = ProjectConfig {
            project: Some(ProjectSettings {
                env: vec!["NODE_ENV=development".into(), "DEBUG=app:*".into()],
                shell: None,
            }),
            ..Default::default()
        };

        let project = Project::new("test".into(), PathBuf::from("/test"), config);
        assert_eq!(project.env_vars().len(), 2);
        assert_eq!(project.env_vars()[0], "NODE_ENV=development");
    }

    #[test]
    fn test_services_yml_path() {
        let config = ProjectConfig {
            dependencies: ProjectDependencies {
                services_yml: Some(PathBuf::from("services.yml")),
                ..Default::default()
            },
            ..Default::default()
        };

        let project = Project::new("test".into(), PathBuf::from("/home/dev/code/test"), config);

        assert_eq!(
            project.services_yml_path(),
            Some(PathBuf::from("/home/dev/code/test/services.yml"))
        );
    }
}
