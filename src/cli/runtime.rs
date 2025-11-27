use anyhow::{Context, Result, bail};
use devobox::domain::{ContainerState, Database};
use devobox::infra::PodmanAdapter;
use devobox::infra::config::load_databases;
use devobox::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct Runtime {
    config_dir: PathBuf,
    databases: Vec<Database>,
    container_service: Arc<ContainerService>,
    orchestrator: Orchestrator,
}

impl Runtime {
    fn new(config_dir: &Path) -> Result<Self> {
        let databases = load_databases(config_dir)?;
        let runtime = Arc::new(PodmanAdapter::new());
        let container_service = Arc::new(ContainerService::new(runtime.clone()));
        let system_service = Arc::new(SystemService::new(runtime));
        let orchestrator = Orchestrator::new(container_service.clone(), system_service);
        Ok(Self {
            config_dir: config_dir.to_path_buf(),
            databases,
            container_service,
            orchestrator,
        })
    }

    fn ensure_dev_container(&self) -> Result<()> {
        self.container_service.ensure_running("devobox")
    }

    fn start_all_dbs(&self) -> Result<()> {
        if self.databases.is_empty() {
            println!("⚠️  Nenhum banco configurado em {:?}", self.config_dir);
            return Ok(());
        }

        for db in &self.databases {
            self.ensure_db_created(db)?;
        }

        let db_names: Vec<String> = self.databases.iter().map(|db| db.name.clone()).collect();
        self.orchestrator.start_all(&db_names)
    }

    fn stop_all_dbs(&self) -> Result<()> {
        if self.databases.is_empty() {
            println!("⚠️  Nenhum banco configurado em {:?}", self.config_dir);
            return Ok(());
        }
        let db_names: Vec<String> = self.databases.iter().map(|db| db.name.clone()).collect();
        self.orchestrator.stop_all(&db_names)
    }

    fn restart_all_dbs(&self) -> Result<()> {
        if self.databases.is_empty() {
            println!("⚠️  Nenhum banco configurado");
            return Ok(());
        }
        let db_names: Vec<String> = self.databases.iter().map(|db| db.name.clone()).collect();
        self.orchestrator.stop_all(&db_names)?;
        self.orchestrator.start_all(&db_names)
    }

    fn start_db(&self, service: &str) -> Result<()> {
        let db = self
            .databases
            .iter()
            .find(|d| d.name == service)
            .context(format!(
                "Banco '{service}' não está listado em databases.yml"
            ))?;

        self.ensure_db_created(db)?;
        self.container_service.start(service)
    }

    fn stop_db(&self, service: &str) -> Result<()> {
        if !self.is_known_db(service) {
            bail!("Banco '{service}' não está listado em databases.yml");
        }
        self.container_service.stop(service)
    }

    fn restart_db(&self, service: &str) -> Result<()> {
        if !self.is_known_db(service) {
            bail!("Banco '{service}' não está listado em databases.yml");
        }
        self.container_service.stop(service)?;
        self.container_service.start(service)
    }

    fn is_known_db(&self, name: &str) -> bool {
        self.databases.iter().any(|db| db.name == name)
    }

    fn status(&self) -> Result<()> {
        println!("📦 Status dos containers:");
        let mut missing = false;

        for name in self.all_containers() {
            let container = self.container_service.get_status(&name)?;
            let state = match container.state {
                devobox::domain::ContainerState::Running => "rodando",
                devobox::domain::ContainerState::Stopped => "parado",
                devobox::domain::ContainerState::NotCreated => {
                    missing = true;
                    "não criado"
                }
            };

            println!("- {:<10} | {}", name, state);
        }

        if missing {
            println!("⚠️  Há containers ausentes. Rode 'devobox builder build'.");
        }

        Ok(())
    }

    fn run_shell(&self, with_dbs: bool, auto_stop: bool) -> Result<()> {
        if with_dbs {
            self.start_all_dbs()?;
        }

        self.ensure_dev_container()?;

        let workdir = container_workdir()?;
        println!("🚀 Entrando no devobox (workdir {:?})", workdir);
        let result = self
            .container_service
            .exec_shell("devobox", workdir.as_deref());

        // Stop all containers on exit if auto_stop is enabled
        if auto_stop {
            self.stop_all_containers()?;
        }

        result
    }

    fn stop_all_containers(&self) -> Result<()> {
        let containers = self.all_containers();
        self.orchestrator.stop_all(&containers)
    }

    fn all_containers(&self) -> Vec<String> {
        let mut names = Vec::with_capacity(self.databases.len() + 1);
        names.push("devobox".to_string());
        names.extend(self.databases.iter().map(|db| db.name.clone()));
        names
    }

    fn cleanup(&self, options: &CleanupOptions) -> Result<()> {
        self.orchestrator.cleanup(options)
    }

    fn ensure_db_created(&self, db: &Database) -> Result<()> {
        let status = self.container_service.get_status(&db.name)?;

        if status.state == ContainerState::NotCreated {
            println!("🆕 Criando container para {}...", db.name);
            self.container_service.recreate(&db.to_spec())?;
        }

        Ok(())
    }
}

pub fn shell(config_dir: &Path, with_dbs: bool, auto_stop: bool) -> Result<()> {
    if !config_dir.exists() {
        println!("⚠️  Ambiente não configurado.");
        println!("🔧 Executando setup inicial automaticamente...\n");

        crate::cli::agent::install(config_dir)?;
    }

    let runtime = Runtime::new(config_dir)?;

    let devobox_status = runtime.container_service.get_status("devobox")?;
    if devobox_status.state == ContainerState::NotCreated {
        println!("⚠️  Container 'devobox' não encontrado.");
        println!("🔧 Construindo ambiente...\n");

        crate::cli::builder::build(config_dir, false)?;
    }

    println!("\n✅ Ambiente pronto! Abrindo shell...\n");

    runtime.run_shell(with_dbs, auto_stop)
}

pub fn up(config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.start_all_dbs()?;
    runtime.ensure_dev_container()
}

pub fn down(config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    for name in runtime.all_containers() {
        runtime.container_service.stop(&name)?;
    }
    println!("✅ Tudo parado");
    Ok(())
}

pub fn status(config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.status()
}

pub fn db_start(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.start_db(name),
        None => runtime.start_all_dbs(),
    }
}

pub fn db_stop(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.stop_db(name),
        None => runtime.stop_all_dbs(),
    }
}

pub fn db_restart(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.restart_db(name),
        None => runtime.restart_all_dbs(),
    }
}

pub fn cleanup(config_dir: &Path, options: &CleanupOptions) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.cleanup(options)
}

fn container_workdir() -> Result<Option<PathBuf>> {
    let pwd = std::env::current_dir()?;
    let home = std::env::var("HOME").unwrap_or_default();
    let home_path = PathBuf::from(&home);
    let code_dir = home_path.join("code");

    if let Ok(stripped) = pwd.strip_prefix(&code_dir) {
        return Ok(Some(PathBuf::from("/home/dev/code").join(stripped)));
    }

    Ok(Some(PathBuf::from("/home/dev")))
}
