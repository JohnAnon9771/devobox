use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use devobox::domain::Database;
use devobox::infra::PodmanAdapter;
use devobox::infra::config::load_databases;
use devobox::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Args)]
pub struct RuntimeCommand {
    #[command(subcommand)]
    pub command: RuntimeAction,
}

#[derive(Subcommand)]
pub enum RuntimeAction {
    /// Abre um shell dentro do container devobox
    Shell {
        /// Inicializa bancos antes de entrar
        #[arg(long)]
        with_dbs: bool,
        /// Para todos os containers ao sair do shell
        #[arg(long)]
        auto_stop: bool,
    },
    /// Sobe devobox e todos os bancos configurados
    Up,
    /// Para todos os containers conhecidos
    Down,
    /// Mostra status geral
    Status,
    /// Controle de bancos de dados
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
    /// Limpa recursos não utilizados do Podman
    Cleanup {
        /// Limpar apenas containers parados
        #[arg(long)]
        containers: bool,
        /// Limpar apenas imagens não utilizadas
        #[arg(long)]
        images: bool,
        /// Limpar apenas volumes órfãos
        #[arg(long)]
        volumes: bool,
        /// Limpar apenas cache de build
        #[arg(long)]
        build_cache: bool,
        /// Limpar tudo (padrão se nenhuma flag especificada)
        #[arg(long)]
        all: bool,
    },
}

#[derive(Subcommand)]
pub enum DbAction {
    Start { service: Option<String> },
    Stop { service: Option<String> },
    Restart { service: Option<String> },
    Status,
}

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
        if !self.is_known_db(service) {
            bail!("Banco '{service}' não está listado em databases.yml");
        }
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

    fn shell(&self, with_dbs: bool, auto_stop: bool) -> Result<()> {
        if with_dbs {
            self.start_all_dbs()?;
        }

        self.ensure_dev_container()?;

        let workdir = container_workdir()?;
        println!("🚀 Entrando no devobox (workdir {:?})", workdir);
        let result = self
            .container_service
            .exec_shell("devobox", workdir.as_deref());

        // Para todos os containers ao sair se auto_stop estiver ativado
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
}

pub fn run(cmd: RuntimeCommand, config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;

    match cmd.command {
        RuntimeAction::Shell {
            with_dbs,
            auto_stop,
        } => runtime.shell(with_dbs, auto_stop),
        RuntimeAction::Up => {
            runtime.start_all_dbs()?;
            runtime.ensure_dev_container()
        }
        RuntimeAction::Down => {
            for name in runtime.all_containers() {
                runtime.container_service.stop(&name)?;
            }
            println!("✅ Tudo parado");
            Ok(())
        }
        RuntimeAction::Status => runtime.status(),
        RuntimeAction::Db { action } => match action {
            DbAction::Start { service } => match service {
                Some(name) => runtime.start_db(&name),
                None => runtime.start_all_dbs(),
            },
            DbAction::Stop { service } => match service {
                Some(name) => runtime.stop_db(&name),
                None => runtime.stop_all_dbs(),
            },
            DbAction::Restart { service } => match service {
                Some(name) => runtime.restart_db(&name),
                None => runtime.restart_all_dbs(),
            },
            DbAction::Status => runtime.status(),
        },
        RuntimeAction::Cleanup {
            containers,
            images,
            volumes,
            build_cache,
            all,
        } => {
            // Se nenhuma flag específica foi fornecida, ou se --all foi especificado, limpa tudo
            let cleanup_all = all || (!containers && !images && !volumes && !build_cache);

            let options = if cleanup_all {
                CleanupOptions::all()
            } else {
                CleanupOptions {
                    containers,
                    images,
                    volumes,
                    build_cache,
                }
            };

            runtime.cleanup(&options)
        }
    }
}

fn container_workdir() -> Result<Option<PathBuf>> {
    let pwd = std::env::current_dir()?;
    let home = std::env::var("HOME").unwrap_or_default();
    let home_path = PathBuf::from(&home);

    if let Ok(stripped) = pwd.strip_prefix(&home_path) {
        return Ok(Some(PathBuf::from("/home/dev").join(stripped)));
    }

    Ok(None)
}
