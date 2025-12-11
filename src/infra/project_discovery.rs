use crate::domain::{Project, ProjectConfig};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Discovers projects in configured directory (default: ~/code)
pub struct ProjectDiscovery {
    base_dir: PathBuf,
}

impl ProjectDiscovery {
    /// Creates a new ProjectDiscovery instance
    ///
    /// # Arguments
    /// * `base_dir` - Optional base directory to scan for projects. Defaults to ~/code
    pub fn new(base_dir: Option<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("code")
        });

        if !base_dir.exists() {
            info!("Diretório de projetos não existe, criando: {:?}", base_dir);
            fs::create_dir_all(&base_dir)
                .with_context(|| format!("Criando diretório de projetos: {:?}", base_dir))?;
        }

        Ok(Self { base_dir })
    }

    /// Lists all projects (directories with devobox.toml)
    ///
    /// Scans the base directory for subdirectories containing a devobox.toml file.
    /// Only direct children are scanned (no recursive search).
    pub fn discover_all(&self) -> Result<Vec<Project>> {
        let mut projects = Vec::new();

        let entries = fs::read_dir(&self.base_dir)
            .with_context(|| format!("Lendo diretório de projetos: {:?}", self.base_dir))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let config_path = path.join("devobox.toml");
            if !config_path.exists() {
                debug!("Ignorando {:?} - não possui devobox.toml", path.file_name());
                continue;
            }

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            match self.load_project_config(&config_path) {
                Ok(config) => {
                    debug!("Projeto encontrado: {}", name);
                    projects.push(Project::new(name, path, config));
                }
                Err(e) => {
                    debug!("Erro ao carregar projeto {}: {}", name, e);
                    // Continue descobrindo outros projetos mesmo se um falhar
                }
            }
        }

        Ok(projects)
    }

    /// Finds a specific project by name
    ///
    /// # Arguments
    /// * `name` - The project name (directory name)
    ///
    /// # Returns
    /// * `Ok(Some(Project))` if project found
    /// * `Ok(None)` if project doesn't exist
    /// * `Err` if there was an error scanning
    pub fn find_project(&self, name: &str) -> Result<Option<Project>> {
        let projects = self.discover_all()?;
        Ok(projects.into_iter().find(|p| p.name == name))
    }

    /// Loads project configuration from a devobox.toml file
    fn load_project_config(&self, path: &Path) -> Result<ProjectConfig> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Lendo configuração do projeto: {:?}", path))?;

        let config: ProjectConfig = toml::from_str(&content)
            .with_context(|| format!("Parsing configuração do projeto: {:?}", path))?;

        Ok(config)
    }

    /// Returns the base directory being scanned
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discovery_empty_directory() {
        let temp = TempDir::new().unwrap();
        let discovery = ProjectDiscovery::new(Some(temp.path().to_path_buf())).unwrap();

        let projects = discovery.discover_all().unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_discovery_project_without_config() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("test-project");
        fs::create_dir(&project_dir).unwrap();

        let discovery = ProjectDiscovery::new(Some(temp.path().to_path_buf())).unwrap();
        let projects = discovery.discover_all().unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_discovery_project_with_config() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("test-project");
        fs::create_dir(&project_dir).unwrap();

        // Create minimal devobox.toml
        fs::write(project_dir.join("devobox.toml"), "[project]\n").unwrap();

        let discovery = ProjectDiscovery::new(Some(temp.path().to_path_buf())).unwrap();
        let projects = discovery.discover_all().unwrap();

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "test-project");
    }

    #[test]
    fn test_discovery_multiple_projects() {
        let temp = TempDir::new().unwrap();

        // Create multiple projects
        for name in &["project-a", "project-b", "project-c"] {
            let project_dir = temp.path().join(name);
            fs::create_dir(&project_dir).unwrap();
            fs::write(project_dir.join("devobox.toml"), "[project]\n").unwrap();
        }

        // Create a directory without config (should be ignored)
        fs::create_dir(temp.path().join("not-a-project")).unwrap();

        let discovery = ProjectDiscovery::new(Some(temp.path().to_path_buf())).unwrap();
        let projects = discovery.discover_all().unwrap();

        assert_eq!(projects.len(), 3);
        assert!(projects.iter().any(|p| p.name == "project-a"));
        assert!(projects.iter().any(|p| p.name == "project-b"));
        assert!(projects.iter().any(|p| p.name == "project-c"));
    }

    #[test]
    fn test_find_project() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("my-app");
        fs::create_dir(&project_dir).unwrap();
        fs::write(
            project_dir.join("devobox.toml"),
            r#"
[project]
env = ["NODE_ENV=development"]
            "#,
        )
        .unwrap();

        let discovery = ProjectDiscovery::new(Some(temp.path().to_path_buf())).unwrap();

        // Find existing project
        let found = discovery.find_project("my-app").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my-app");

        // Try to find non-existing project
        let not_found = discovery.find_project("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_load_project_config_with_env() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("env-test");
        fs::create_dir(&project_dir).unwrap();

        let config_content = r#"
[project]
env = ["NODE_ENV=development", "DEBUG=app:*"]
shell = "zsh"

[dependencies]
services_yml = "services.yml"
include_projects = ["../other-project"]
"#;
        fs::write(project_dir.join("devobox.toml"), config_content).unwrap();

        let discovery = ProjectDiscovery::new(Some(temp.path().to_path_buf())).unwrap();
        let project = discovery.find_project("env-test").unwrap().unwrap();

        assert_eq!(project.env_vars().len(), 2);
        assert_eq!(project.env_vars()[0], "NODE_ENV=development");
        assert_eq!(project.shell(), Some("zsh"));
    }
}
