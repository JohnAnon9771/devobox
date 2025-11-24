mod cli;

use anyhow::Result;
use clap::{Parser, Subcommand};
use devobox::services::CleanupOptions;

#[derive(Parser)]
#[command(
    name = "devobox",
    about = "Gerenciador de ambiente de desenvolvimento containerizado"
)]
struct Cli {
    /// Diretório de configuração (default: ~/.config/devobox)
    #[arg(long, env, default_value_os_t = devobox::infra::config::default_config_dir())]
    config_dir: std::path::PathBuf,

    /// Inicializar bancos de dados antes de abrir o shell (apenas quando nenhum subcomando é fornecido)
    #[arg(long, short = 'd')]
    with_dbs: bool,

    /// Parar todos os containers ao sair do shell (apenas quando nenhum subcomando é fornecido)
    #[arg(long)]
    auto_stop: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup completo: instala configurações e constrói ambiente
    Init {
        /// Pular limpeza automática de recursos durante o build
        #[arg(long)]
        skip_cleanup: bool,
    },
    /// Reconstrói a imagem e recria containers
    Rebuild {
        /// Pular limpeza automática de recursos
        #[arg(long)]
        skip_cleanup: bool,
    },
    /// Abre um shell dentro do container devobox
    Shell {
        /// Inicializa bancos antes de entrar
        #[arg(long)]
        with_dbs: bool,
        /// Para todos os containers ao sair do shell
        #[arg(long)]
        auto_stop: bool,
    },
    /// Abre shell com bancos de dados (atalho para 'shell --with-dbs')
    Dev {
        /// Para todos os containers ao sair do shell
        #[arg(long)]
        auto_stop: bool,
    },
    /// Sobe devobox e todos os bancos configurados
    #[command(alias = "start")]
    Up,
    /// Para todos os containers
    #[command(alias = "stop")]
    Down,
    /// Mostra status de todos os containers
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
enum DbAction {
    /// Inicia banco(s) de dados
    Start {
        /// Nome do banco específico (opcional)
        service: Option<String>,
    },
    /// Para banco(s) de dados
    Stop {
        /// Nome do banco específico (opcional)
        service: Option<String>,
    },
    /// Reinicia banco(s) de dados
    Restart {
        /// Nome do banco específico (opcional)
        service: Option<String>,
    },
    /// Mostra status dos bancos
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // Default behavior: open shell
            cli::runtime::shell(&cli.config_dir, cli.with_dbs, cli.auto_stop)
        }
        Some(Commands::Init { skip_cleanup }) => {
            println!("📦 Passo 1/2: Instalando configurações...");
            cli::agent::install(&cli.config_dir)?;

            println!("\n📦 Passo 2/2: Construindo ambiente...");
            cli::builder::build(&cli.config_dir, skip_cleanup)?;

            println!("\n✅ Setup completo! Use 'devobox' para abrir o shell.");
            Ok(())
        }
        Some(Commands::Rebuild { skip_cleanup }) => {
            cli::builder::build(&cli.config_dir, skip_cleanup)
        }
        Some(Commands::Shell {
            with_dbs,
            auto_stop,
        }) => cli::runtime::shell(&cli.config_dir, with_dbs, auto_stop),
        Some(Commands::Dev { auto_stop }) => cli::runtime::shell(&cli.config_dir, true, auto_stop),
        Some(Commands::Up) => cli::runtime::up(&cli.config_dir),
        Some(Commands::Down) => cli::runtime::down(&cli.config_dir),
        Some(Commands::Status) => cli::runtime::status(&cli.config_dir),
        Some(Commands::Db { action }) => match action {
            DbAction::Start { service } => {
                cli::runtime::db_start(&cli.config_dir, service.as_deref())
            }
            DbAction::Stop { service } => {
                cli::runtime::db_stop(&cli.config_dir, service.as_deref())
            }
            DbAction::Restart { service } => {
                cli::runtime::db_restart(&cli.config_dir, service.as_deref())
            }
            DbAction::Status => cli::runtime::status(&cli.config_dir),
        },
        Some(Commands::Cleanup {
            containers,
            images,
            volumes,
            build_cache,
            all,
        }) => {
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
            cli::runtime::cleanup(&cli.config_dir, &options)
        }
    }
}
