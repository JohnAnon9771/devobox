use crate::config::{Database, load_databases};
use crate::podman::{
    container_exists, container_running, exec_shell, start_container, stop_container,
};
use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use std::path::{Path, PathBuf};

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
}

impl Runtime {
    fn new(config_dir: &Path) -> Result<Self> {
        let databases = load_databases(config_dir)?;
        Ok(Self {
            config_dir: config_dir.to_path_buf(),
            databases,
        })
    }

    fn ensure_dev_container(&self) -> Result<()> {
        if container_running("devobox")? {
            return Ok(());
        }

        if container_exists("devobox")? {
            println!("🔌 Iniciando devobox...");
            start_container("devobox")?;
            return Ok(());
        }

        bail!("Container devobox não encontrado. Execute 'devobox builder build' para criá-lo.");
    }

    fn start_all_dbs(&self) -> Result<()> {
        if self.databases.is_empty() {
            println!("⚠️  Nenhum banco configurado em {:?}", self.config_dir);
            return Ok(());
        }

        for db in &self.databases {
            self.start_db(&db.name)?;
        }
        Ok(())
    }

    fn stop_all_dbs(&self) -> Result<()> {
        if self.databases.is_empty() {
            println!("⚠️  Nenhum banco configurado em {:?}", self.config_dir);
            return Ok(());
        }

        for db in &self.databases {
            self.stop_db(&db.name)?;
        }
        Ok(())
    }

    fn restart_all_dbs(&self) -> Result<()> {
        self.stop_all_dbs()?;
        self.start_all_dbs()
    }

    fn start_db(&self, service: &str) -> Result<()> {
        if !self.is_known_db(service) {
            bail!("Banco '{service}' não está listado em databases.yml");
        }

        if container_running(service)? {
            println!("⚠️  {service} já está rodando");
        } else if container_exists(service)? {
            println!("🔌 Iniciando {service}...");
            start_container(service)?;
        } else {
            println!("⚠️  Container {service} não existe. Rode 'devobox builder build' primeiro.");
        }

        Ok(())
    }

    fn stop_db(&self, service: &str) -> Result<()> {
        if !self.is_known_db(service) {
            bail!("Banco '{service}' não está listado em databases.yml");
        }

        if container_running(service)? {
            println!("💤 Parando {service}...");
            stop_container(service)?;
        } else {
            println!("⚠️  {service} já está parado ou não foi criado");
        }

        Ok(())
    }

    fn status(&self) -> Result<()> {
        println!("📦 Status dos containers:");
        let mut missing = false;

        for name in self.all_containers() {
            let state = if container_exists(&name)? {
                if container_running(&name)? {
                    "rodando"
                } else {
                    "parado"
                }
            } else {
                missing = true;
                "não criado"
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
        exec_shell("devobox", workdir.as_deref())
    }

    fn all_containers(&self) -> Vec<String> {
        let mut names = Vec::with_capacity(self.databases.len() + 1);
        names.push("devobox".to_string());
        names.extend(self.databases.iter().map(|db| db.name.clone()));
        names
    }

    fn is_known_db(&self, name: &str) -> bool {
        self.databases.iter().any(|db| db.name == name)
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
                if container_running(&name)? {
                    stop_container(&name)?;
                }
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
                Some(name) => {
                    runtime.stop_db(&name)?;
                    runtime.start_db(&name)
                }
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
