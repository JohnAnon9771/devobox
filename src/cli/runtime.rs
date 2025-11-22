use anyhow::Result;
use clap::{Args, Subcommand};
use devobox::domain::Database;
use devobox::infra::PodmanAdapter;
use devobox::infra::config::load_databases;
use devobox::services::{ContainerService, DatabaseService};
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
    container_service: ContainerService,
    db_service: DatabaseService,
}

impl Runtime {
    fn new(config_dir: &Path) -> Result<Self> {
        let databases = load_databases(config_dir)?;
        let runtime = Arc::new(PodmanAdapter::new());
        Ok(Self {
            config_dir: config_dir.to_path_buf(),
            databases,
            container_service: ContainerService::new(runtime.clone()),
            db_service: DatabaseService::new(runtime),
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
        self.db_service.start_all(&self.databases)
    }

    fn stop_all_dbs(&self) -> Result<()> {
        if self.databases.is_empty() {
            println!("⚠️  Nenhum banco configurado em {:?}", self.config_dir);
            return Ok(());
        }
        self.db_service.stop_all(&self.databases)
    }

    fn restart_all_dbs(&self) -> Result<()> {
        self.db_service.restart_all(&self.databases)
    }

    fn start_db(&self, service: &str) -> Result<()> {
        self.db_service.start(service, &self.databases)
    }

    fn stop_db(&self, service: &str) -> Result<()> {
        self.db_service.stop(service, &self.databases)
    }

    fn restart_db(&self, service: &str) -> Result<()> {
        self.db_service.restart(service, &self.databases)
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

    fn shell(&self, with_dbs: bool) -> Result<()> {
        if with_dbs {
            self.start_all_dbs()?;
        }

        self.ensure_dev_container()?;

        let workdir = container_workdir()?;
        println!("🚀 Entrando no devobox (workdir {:?})", workdir);
        self.container_service
            .exec_shell("devobox", workdir.as_deref())
    }

    fn all_containers(&self) -> Vec<String> {
        let mut names = Vec::with_capacity(self.databases.len() + 1);
        names.push("devobox".to_string());
        names.extend(self.databases.iter().map(|db| db.name.clone()));
        names
    }
}

pub fn run(cmd: RuntimeCommand, config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;

    match cmd.command {
        RuntimeAction::Shell { with_dbs } => runtime.shell(with_dbs),
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
