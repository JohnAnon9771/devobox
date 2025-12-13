use crate::domain::{Project, ProjectConfig, Service};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml;
use tracing::{info, warn};

#[derive(Deserialize, Debug)]
pub struct MiseConfig {
    pub tools: std::collections::HashMap<String, String>,
}

pub fn default_config_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/dev"))
        .join(".config/devobox")
}

pub fn ensure_config_dir(config_dir: &Path) -> Result<()> {
    fs::create_dir_all(config_dir).with_context(|| format!("criando {:?}", config_dir))
}

pub const DEFAULT_DEVOBOX_TOML_NAME: &str = "devobox.toml";
pub const MISE_TOML: &str = include_str!("../../config/mise.toml");
pub const STARSHIP_TOML: &str = include_str!("../../config/starship.toml");

#[derive(Deserialize, Debug, Default)]
pub struct PathsConfig {
    pub containerfile: Option<PathBuf>,
    pub mise_toml: Option<PathBuf>,
    pub starship_toml: Option<PathBuf>,
}

#[derive(Deserialize, Debug, Default)]
pub struct BuildConfig {
    pub image_name: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct ContainerConfig {
    pub name: Option<String>,
    pub workdir: Option<PathBuf>,
}

#[derive(Deserialize, Debug, Default)]
pub struct DependenciesConfig {
    pub include_projects: Option<Vec<PathBuf>>,
}

#[derive(Deserialize, Debug, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub container: ContainerConfig,
    #[serde(default)]
    pub dependencies: DependenciesConfig,
    /// Services defined inline as [services.NAME]
    #[serde(default)]
    pub services: Option<HashMap<String, Service>>,
}

impl AppConfig {
    /// Merges another AppConfig into self.
    /// Values from `other` overwrite values in `self` if present.
    pub fn merge(&mut self, other: AppConfig) {
        if let Some(cf) = other.paths.containerfile {
            self.paths.containerfile = Some(cf);
        }
        if let Some(m) = other.paths.mise_toml {
            self.paths.mise_toml = Some(m);
        }
        if let Some(s) = other.paths.starship_toml {
            self.paths.starship_toml = Some(s);
        }
        if let Some(name) = other.build.image_name {
            self.build.image_name = Some(name);
        }
        if let Some(name) = other.container.name {
            self.container.name = Some(name);
        }
        if let Some(wd) = other.container.workdir {
            self.container.workdir = Some(wd);
        }
        if let Some(deps) = other.dependencies.include_projects {
            // Merge dependencies: append unique ones or overwrite?
            // Appending seems safer to gather all deps.
            let mut current = self
                .dependencies
                .include_projects
                .take()
                .unwrap_or_default();
            for dep in deps {
                if !current.contains(&dep) {
                    current.push(dep);
                }
            }
            self.dependencies.include_projects = Some(current);
        }

        // Merge services
        if let Some(other_services) = other.services {
            match &mut self.services {
                Some(existing) => {
                    // Services with same name in 'other' overwrite existing
                    for (name, service) in other_services {
                        existing.insert(name, service);
                    }
                }
                None => {
                    self.services = Some(other_services);
                }
            }
        }
    }
}

/// Converts services HashMap to Vec<Service> with validation
fn services_from_hashmap(services_map: &HashMap<String, Service>) -> Result<Vec<Service>> {
    let mut services = Vec::new();

    for (name, service) in services_map {
        // Validate service name
        if name.trim().is_empty() {
            bail!("Nome de serviço vazio encontrado");
        }

        // Validate name format (container name restrictions)
        let first_char = name.chars().next().unwrap();
        if !first_char.is_alphanumeric() {
            bail!(
                "Nome de serviço '{}' deve começar com letra ou número",
                name
            );
        }

        for c in name.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '.' && c != '-' {
                bail!(
                    "Nome de serviço '{}' contém caractere inválido '{}'",
                    name,
                    c
                );
            }
        }

        // Validate image
        if service.image.trim().is_empty() {
            bail!("Serviço '{}' sem campo 'image'", name);
        }

        services.push(service.clone().with_name(name.clone()));
    }

    Ok(services)
}

pub fn resolve_all_services(start_dir: &Path, start_config: &AppConfig) -> Result<Vec<Service>> {
    let mut all_services = Vec::new();
    let mut service_names = HashSet::new();
    let mut visited_paths = HashSet::new();

    visited_paths.insert(fs::canonicalize(start_dir).unwrap_or(start_dir.to_path_buf()));

    // Helper to add services with duplicate detection
    let mut add_services = |services: Vec<Service>| -> Result<()> {
        for service in services {
            if !service_names.insert(service.name.clone()) {
                warn!("  Serviço duplicado ignorado: {}", service.name);
                continue;
            }
            all_services.push(service);
        }
        Ok(())
    };

    // 1. Load services from current config
    if let Some(services_map) = &start_config.services {
        info!(
            "  Carregando {} serviço(s) da configuração atual...",
            services_map.len()
        );
        let services = services_from_hashmap(services_map)?;
        add_services(services)?;
    }

    // 2. Load services from dependencies
    if let Some(deps) = &start_config.dependencies.include_projects {
        for relative_path in deps {
            let project_path = start_dir.join(relative_path);
            let canonical_path = match fs::canonicalize(&project_path) {
                Ok(p) => p,
                Err(_) => {
                    warn!("  Caminho de dependência inválido: {:?}", project_path);
                    continue;
                }
            };

            if !visited_paths.insert(canonical_path.clone()) {
                continue;
            }

            let dep_config = match load_app_config(&canonical_path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    warn!(
                        "  Erro ao carregar config de dependência em {:?}: {}",
                        canonical_path, e
                    );
                    continue;
                }
            };

            if let Some(dep_services_map) = &dep_config.services {
                info!(
                    "  Carregando {} serviço(s) de dependência {:?}...",
                    dep_services_map.len(),
                    canonical_path
                );
                let services = services_from_hashmap(dep_services_map)?;
                add_services(services)?;
            }
        }
    }

    Ok(all_services)
}

pub fn install_default_config(target_dir: &Path) -> Result<()> {
    ensure_config_dir(target_dir)?;

    let files = [
        (
            "Containerfile",
            include_str!("../../config/default_containerfile.dockerfile"),
        ),
        ("mise.toml", MISE_TOML),
        ("starship.toml", STARSHIP_TOML),
        (
            DEFAULT_DEVOBOX_TOML_NAME,
            include_str!("../../config/default_devobox.toml"),
        ),
    ];

    for (name, content) in files {
        let target = target_dir.join(name);

        if target.exists() {
            continue;
        }

        fs::write(&target, content)
            .with_context(|| format!("escrevendo template em {:?}", target))?;
    }

    Ok(())
}

pub fn load_mise_config(path: &Path) -> Result<MiseConfig> {
    if !path.exists() {
        bail!("mise.toml não encontrado em {:?}", path);
    }

    let content = fs::read_to_string(path).with_context(|| format!("lendo {:?}", path))?;
    let config: MiseConfig =
        toml::from_str(&content).with_context(|| format!("parse de {:?}", path))?;

    Ok(config)
}

pub fn containerfile_path(config_dir: &Path) -> PathBuf {
    config_dir.join("Containerfile")
}

pub fn read_containerfile_content(config_dir: &Path) -> Result<String> {
    let path = containerfile_path(config_dir);
    fs::read_to_string(&path).with_context(|| format!("lendo Containerfile em {:?}", path))
}

pub fn load_app_config(config_dir: &Path) -> Result<AppConfig> {
    let global_config_path = config_dir.join(DEFAULT_DEVOBOX_TOML_NAME);
    let mut app_config = AppConfig::default();

    if global_config_path.exists() {
        let content = fs::read_to_string(&global_config_path)
            .with_context(|| format!("lendo config global em {:?}", global_config_path))?;
        let global_app_config: AppConfig = toml::from_str(&content)
            .with_context(|| format!("parse de config global em {:?}", global_config_path))?;
        app_config = global_app_config;
    }

    let local_config_path = PathBuf::from("./").join(DEFAULT_DEVOBOX_TOML_NAME); // Check current working directory
    if local_config_path.exists() {
        let content = fs::read_to_string(&local_config_path)
            .with_context(|| format!("lendo config local em {:?}", local_config_path))?;
        let local_app_config: AppConfig = toml::from_str(&content)
            .with_context(|| format!("parse de config local em {:?}", local_config_path))?;
        app_config.merge(local_app_config);
    }

    // Default values if not set in any config
    if app_config.paths.containerfile.is_none() {
        app_config.paths.containerfile = Some(PathBuf::from("Containerfile"));
    }
    if app_config.paths.mise_toml.is_none() {
        app_config.paths.mise_toml = Some(PathBuf::from("mise.toml"));
    }
    if app_config.paths.starship_toml.is_none() {
        app_config.paths.starship_toml = Some(PathBuf::from("starship.toml"));
    }
    if app_config.build.image_name.is_none() {
        app_config.build.image_name = Some("devobox-img".to_string());
    }
    if app_config.container.name.is_none() {
        app_config.container.name = Some("devobox".to_string());
    }
    if app_config.container.workdir.is_none() {
        app_config.container.workdir = Some(PathBuf::from("/home/dev"));
    }

    Ok(app_config)
}

/// Resolves services for a specific project
///
/// Loads the project's own services.yml and any services from project dependencies.
/// This function is used when activating a project workspace to determine which
/// services need to be started.
///
/// # Arguments
/// * `project` - The project to resolve services for
/// * `_config_dir` - The global config directory (currently unused but kept for future use)
///
/// # Returns
/// * `Ok(Vec<Service>)` - List of all services for the project
/// * `Err` - If there was an error loading services
pub fn resolve_project_services(project: &Project, _config_dir: &Path) -> Result<Vec<Service>> {
    let mut all_services = Vec::new();
    let mut service_names = HashSet::new();
    let mut visited_paths = HashSet::new();

    visited_paths.insert(fs::canonicalize(&project.path).unwrap_or_else(|_| project.path.clone()));

    // Helper to add services with duplicate detection
    let mut add_services = |services: Vec<Service>| -> Result<()> {
        for service in services {
            if !service_names.insert(service.name.clone()) {
                warn!("  Serviço duplicado ignorado: {}", service.name);
                continue;
            }
            all_services.push(service);
        }
        Ok(())
    };

    // 1. Load project's own services
    if let Some(services_map) = &project.config.services {
        info!(
            "  Carregando {} serviço(s) do projeto...",
            services_map.len()
        );
        let services = services_from_hashmap(services_map)?;
        add_services(services)?;
    }

    // 2. Load services from project dependencies
    for relative_path in &project.config.dependencies.include_projects {
        let dep_path = project.path.join(relative_path);
        let canonical_path = match fs::canonicalize(&dep_path) {
            Ok(p) => p,
            Err(_) => {
                warn!("  Caminho de dependência inválido: {:?}", dep_path);
                continue;
            }
        };

        if !visited_paths.insert(canonical_path.clone()) {
            continue;
        }

        let dep_config_path = canonical_path.join("devobox.toml");
        if dep_config_path.exists() {
            match fs::read_to_string(&dep_config_path) {
                Ok(content) => match toml::from_str::<ProjectConfig>(&content) {
                    Ok(dep_config) => {
                        if let Some(dep_services_map) = &dep_config.services {
                            info!(
                                "  Carregando {} serviço(s) de dependência: {:?}...",
                                dep_services_map.len(),
                                dep_config_path
                            );
                            let services = services_from_hashmap(dep_services_map)?;
                            add_services(services)?;
                        }
                    }
                    Err(e) => warn!("  Erro ao fazer parse de {:?}: {}", dep_config_path, e),
                },
                Err(e) => warn!("  Erro ao ler {:?}: {}", dep_config_path, e),
            }
        }
    }

    Ok(all_services)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_services_from_toml() {
        let toml = r#"
[services.pg]
image = "postgres:15"
type = "database"
ports = ["5432:5432"]
env = ["POSTGRES_PASSWORD=dev"]

[services.redis]
image = "redis:7"
"#;

        let config: AppConfig = toml::from_str(toml).unwrap();
        let services = services_from_hashmap(config.services.as_ref().unwrap()).unwrap();

        assert_eq!(services.len(), 2);

        let pg = services.iter().find(|s| s.name == "pg").unwrap();
        assert_eq!(pg.image, "postgres:15");
        assert_eq!(pg.env, vec!["POSTGRES_PASSWORD=dev"]);

        let redis = services.iter().find(|s| s.name == "redis").unwrap();
        assert_eq!(redis.image, "redis:7");
    }

    #[test]
    fn parses_service_with_minimal_fields() {
        let toml = r#"
[services.minimal]
image = "minimal:latest"
"#;

        let config: AppConfig = toml::from_str(toml).unwrap();
        let services = services_from_hashmap(config.services.as_ref().unwrap()).unwrap();

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "minimal");
        assert_eq!(services[0].image, "minimal:latest");
    }

    #[test]
    fn rejects_missing_image() {
        let toml = r#"
[services.pg]
ports = ["5432:5432"]
"#;

        // Should fail at deserialization (image is required)
        let result = toml::from_str::<AppConfig>(toml);
        assert!(result.is_err());
    }

    #[test]
    fn validates_service_name() {
        use crate::domain::ServiceKind;
        let mut services_map = HashMap::new();
        services_map.insert(
            "".to_string(),
            Service {
                name: String::new(),
                image: "test".to_string(),
                kind: ServiceKind::default(),
                ports: vec![],
                env: vec![],
                volumes: vec![],
                healthcheck_command: None,
                healthcheck_interval: None,
                healthcheck_timeout: None,
                healthcheck_retries: None,
            },
        );

        let result = services_from_hashmap(&services_map);
        assert!(result.is_err());
    }

    #[test]
    fn merges_services_from_configs() {
        use crate::domain::ServiceKind;
        let mut base = AppConfig::default();
        let mut base_services = HashMap::new();
        base_services.insert(
            "pg".to_string(),
            Service {
                name: String::new(),
                image: "postgres:15".to_string(),
                kind: ServiceKind::Database,
                ports: vec![],
                env: vec![],
                volumes: vec![],
                healthcheck_command: None,
                healthcheck_interval: None,
                healthcheck_timeout: None,
                healthcheck_retries: None,
            },
        );
        base.services = Some(base_services);

        let mut override_config = AppConfig::default();
        let mut override_services = HashMap::new();
        override_services.insert(
            "redis".to_string(),
            Service {
                name: String::new(),
                image: "redis:7".to_string(),
                kind: ServiceKind::Database,
                ports: vec![],
                env: vec![],
                volumes: vec![],
                healthcheck_command: None,
                healthcheck_interval: None,
                healthcheck_timeout: None,
                healthcheck_retries: None,
            },
        );
        override_config.services = Some(override_services);

        base.merge(override_config);

        let services_map = base.services.unwrap();
        assert_eq!(services_map.len(), 2);
        assert!(services_map.contains_key("pg"));
        assert!(services_map.contains_key("redis"));
    }

    #[test]
    fn installs_default_config() {
        let temp_dir = std::env::temp_dir().join("devobox_test_install");
        let target_dir = temp_dir.join("target");

        // Ensure clean state
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir).unwrap();
        }

        install_default_config(&target_dir).unwrap();

        assert!(target_dir.join("mise.toml").exists());
        assert!(target_dir.join("Containerfile").exists());

        // Verify that devobox.toml has services
        let devobox_toml_path = target_dir.join(DEFAULT_DEVOBOX_TOML_NAME);
        assert!(devobox_toml_path.exists());
        let content = fs::read_to_string(devobox_toml_path).unwrap();
        assert!(content.contains("[services."));

        // Verify content matches embedded content
        assert_eq!(
            fs::read_to_string(target_dir.join("mise.toml")).unwrap(),
            MISE_TOML
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn parses_mise_toml() {
        let toml = r#"
[tools]
ruby = "3.2"
node = "20"
"#;
        let temp_dir = std::env::temp_dir().join("devobox_test_mise_parse");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("mise.toml"), toml).unwrap();

        let config = load_mise_config(&temp_dir.join("mise.toml")).unwrap();
        assert_eq!(config.tools.get("ruby").unwrap(), "3.2");
        assert_eq!(config.tools.get("node").unwrap(), "20");

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
