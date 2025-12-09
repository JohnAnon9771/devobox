use crate::domain::{Project, Service};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use toml;
use tracing::{info, warn};

#[derive(Deserialize)]
#[serde(untagged)]
enum ServiceDocument {
    Root { services: Vec<Service> },
    List(Vec<Service>),
}

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
pub const DEFAULT_CONTAINERFILE_NAME: &str = "default_containerfile.dockerfile";
pub const SERVICES_YML: &str = include_str!("../../config/default_services.yml");
pub const MISE_TOML: &str = include_str!("../../config/mise.toml");
pub const STARSHIP_TOML: &str = include_str!("../../config/starship.toml");

#[derive(Deserialize, Debug, Default)]
pub struct PathsConfig {
    pub containerfile: Option<PathBuf>,
    pub services_yml: Option<PathBuf>,
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
}

impl AppConfig {
    /// Merges another AppConfig into self.
    /// Values from `other` overwrite values in `self` if present.
    pub fn merge(&mut self, other: AppConfig) {
        if let Some(cf) = other.paths.containerfile {
            self.paths.containerfile = Some(cf);
        }
        if let Some(sv) = other.paths.services_yml {
            self.paths.services_yml = Some(sv);
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
    }
}

pub fn resolve_all_services(start_dir: &Path, start_config: &AppConfig) -> Result<Vec<Service>> {
    let mut all_services = Vec::new();
    let mut visited_paths = HashSet::new();

    // Resolve starting path
    visited_paths.insert(fs::canonicalize(start_dir).unwrap_or(start_dir.to_path_buf()));

    // 1. Load services from current project
    if let Some(services_yml_name) = &start_config.paths.services_yml {
        let local_services_path = start_dir.join(services_yml_name);
        if local_services_path.exists() {
            info!("   Carregando serviços de {:?}...", local_services_path);
            let services = load_services(&local_services_path)?;
            all_services.extend(services);
        }
    }

    // 2. Iterate over dependencies
    if let Some(deps) = &start_config.dependencies.include_projects {
        for relative_path in deps {
            let project_path = start_dir.join(relative_path);
            let canonical_path = match fs::canonicalize(&project_path) {
                Ok(p) => p,
                Err(_) => {
                    warn!(
                        "  Caminho de dependência inválido ou não encontrado: {:?}",
                        project_path
                    );
                    continue;
                }
            };

            if !visited_paths.insert(canonical_path.clone()) {
                continue; // Already visited
            }

            // Load dependency config (without merging with global again, ideally, but reuse load_app_config for simplicity)
            // We want the config specifically for THAT directory to find its services.yml
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

            let dep_services_path = canonical_path.join(
                dep_config
                    .paths
                    .services_yml
                    .unwrap_or(PathBuf::from("services.yml")),
            );

            if dep_services_path.exists() {
                info!("   Carregando dependência de {:?}...", dep_services_path);
                match load_services(&dep_services_path) {
                    Ok(mut services) => all_services.append(&mut services),
                    Err(e) => warn!(
                        "  Erro ao ler serviços de dependência {:?}: {}",
                        dep_services_path, e
                    ),
                }
            }
        }
    }

    Ok(all_services)
}

pub fn install_default_config(target_dir: &Path) -> Result<()> {
    ensure_config_dir(target_dir)?;

    let files = [
        (
            DEFAULT_CONTAINERFILE_NAME,
            include_str!("../../config/default_containerfile.dockerfile"),
        ),
        ("services.yml", SERVICES_YML),
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

pub fn load_services(path: &Path) -> Result<Vec<Service>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).with_context(|| format!("lendo {:?}", path))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    parse_services(&content, path)
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

fn parse_services(content: &str, path: &Path) -> Result<Vec<Service>> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let doc: ServiceDocument =
        serde_yml::from_str(content).with_context(|| format!("parse de {:?}", path))?;

    let services = match doc {
        ServiceDocument::Root { services } => services,
        ServiceDocument::List(list) => list,
    };

    let mut names = HashSet::new();

    for (idx, svc) in services.iter().enumerate() {
        if svc.name.trim().is_empty() {
            bail!("Entrada {} em {:?} sem 'name'", idx + 1, path);
        }

        if svc.image.trim().is_empty() {
            bail!("Entrada {} em {:?} sem 'image'", idx + 1, path);
        }

        if !names.insert(svc.name.clone()) {
            bail!(
                "Entrada {} em {:?} duplicou o nome '{}'",
                idx + 1,
                path,
                svc.name
            );
        }
    }

    Ok(services)
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
    if app_config.paths.services_yml.is_none() {
        app_config.paths.services_yml = Some(PathBuf::from("services.yml"));
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
    let mut visited_paths = HashSet::new();

    // Mark project path as visited
    visited_paths.insert(fs::canonicalize(&project.path).unwrap_or_else(|_| project.path.clone()));

    // 1. Load project's own services if configured
    if let Some(services_yml_path) = project.services_yml_path() {
        if services_yml_path.exists() {
            info!("  Carregando serviços de {:?}...", services_yml_path);
            let services = load_services(&services_yml_path)?;
            all_services.extend(services);
        } else {
            warn!(
                "  Arquivo de serviços configurado mas não encontrado: {:?}",
                services_yml_path
            );
        }
    }

    // 2. Load services from project dependencies
    for relative_path in &project.config.dependencies.include_projects {
        let dep_path = project.path.join(relative_path);
        let canonical_path = match fs::canonicalize(&dep_path) {
            Ok(p) => p,
            Err(_) => {
                warn!(
                    "  Caminho de dependência inválido ou não encontrado: {:?}",
                    dep_path
                );
                continue;
            }
        };

        if !visited_paths.insert(canonical_path.clone()) {
            continue; // Already visited
        }

        // Try to load services.yml from dependency
        let dep_services_path = canonical_path.join("services.yml");
        if dep_services_path.exists() {
            info!(
                "  Carregando serviços de dependência: {:?}...",
                dep_services_path
            );
            match load_services(&dep_services_path) {
                Ok(mut services) => all_services.append(&mut services),
                Err(e) => warn!(
                    "  Erro ao ler serviços de dependência {:?}: {}",
                    dep_services_path, e
                ),
            }
        }
    }

    Ok(all_services)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_root_databases_key() {
        let yaml = r#"
services:
  - name: pg
    image: postgres:15
    ports: ["5432:5432"]
    env:
      - POSTGRES_PASSWORD=dev
    volumes:
      - /var/lib/postgresql/data
  - name: redis
    image: docker.io/redis:7
"#;

        let dbs = parse_services(yaml, Path::new("services.yml")).unwrap();
        assert_eq!(dbs.len(), 2);
        assert_eq!(dbs[0].name, "pg");
        assert_eq!(dbs[0].env, vec!["POSTGRES_PASSWORD=dev".to_string()]);
        assert_eq!(dbs[0].volumes, vec!["/var/lib/postgresql/data".to_string()]);
        assert_eq!(dbs[1].ports, Vec::<String>::new());
        assert!(dbs[1].volumes.is_empty());
    }

    #[test]
    fn parses_list_style() {
        let yaml = r#"
- name: pg
  image: postgres:15
  ports:
    - "5432:5432"
"#;

        let dbs = parse_services(yaml, Path::new("services.yml")).unwrap();
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].ports, vec!["5432:5432".to_string()]);
    }

    #[test]
    fn rejects_duplicate_names() {
        let yaml = r#"
services:
  - name: pg
    image: postgres:15
  - name: pg
    image: postgres:16
"#;

        let err = parse_services(yaml, Path::new("services.yml")).unwrap_err();
        assert!(err.to_string().contains("duplicou o nome"));
    }

    #[test]
    fn rejects_missing_required_fields() {
        let yaml = r#"
- name: ""
  image: postgres:15
"#;

        let err = parse_services(yaml, Path::new("services.yml")).unwrap_err();
        assert!(err.to_string().contains("sem 'name'"));
    }

    #[test]
    fn empty_file_is_allowed() {
        let parsed = parse_services("   \n", Path::new("services.yml"));
        assert_eq!(parsed.unwrap().len(), 0);
    }

    #[test]
    fn rejects_missing_image() {
        let yaml = r#"
- name: pg
  image: ""
"#;

        let err = parse_services(yaml, Path::new("services.yml")).unwrap_err();
        assert!(err.to_string().contains("sem 'image'"));
    }

    #[test]
    fn parses_database_with_minimal_fields() {
        let yaml = r#"
- name: minimal
  image: minimal:latest
"#;

        let dbs = parse_services(yaml, Path::new("services.yml")).unwrap();
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].name, "minimal");
        assert_eq!(dbs[0].image, "minimal:latest");
        assert!(dbs[0].ports.is_empty());
        assert!(dbs[0].env.is_empty());
        assert!(dbs[0].volumes.is_empty());
    }

    #[test]
    fn parses_multiple_databases() {
        let yaml = r#"
services:
  - name: db1
    image: postgres:15
  - name: db2
    image: mysql:8
  - name: db3
    image: redis:7
"#;

        let dbs = parse_services(yaml, Path::new("services.yml")).unwrap();
        assert_eq!(dbs.len(), 3);
        assert_eq!(dbs[0].name, "db1");
        assert_eq!(dbs[1].name, "db2");
        assert_eq!(dbs[2].name, "db3");
    }

    #[test]
    fn preserves_database_order() {
        let yaml = r#"
- name: first
  image: first:1
- name: second
  image: second:2
- name: third
  image: third:3
"#;

        let dbs = parse_services(yaml, Path::new("services.yml")).unwrap();
        assert_eq!(dbs[0].name, "first");
        assert_eq!(dbs[1].name, "second");
        assert_eq!(dbs[2].name, "third");
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
        assert!(target_dir.join(DEFAULT_CONTAINERFILE_NAME).exists());
        assert!(target_dir.join("services.yml").exists());

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
