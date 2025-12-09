mod cli;

use anyhow::Result;
use clap::{Parser, Subcommand};
use devobox::domain::ServiceKind;
use devobox::services::CleanupOptions;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

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

    /// Habilita logs detalhados (debug level)
    #[arg(long, short = 'v', global = true)]
    verbose: bool,

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
    /// Instala apenas os arquivos de configuração (sem build)
    Install,
    /// Constrói a imagem e cria containers
    Build {
        /// Pular limpeza automática de recursos
        #[arg(long)]
        skip_cleanup: bool,
    },
    /// Reconstrói a imagem e recria containers (alias de 'build')
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
    Up {
        /// Iniciar apenas bancos de dados
        #[arg(long)]
        dbs_only: bool,
        /// Iniciar apenas serviços genéricos (não bancos)
        #[arg(long)]
        services_only: bool,
    },
    /// Para todos os containers
    #[command(alias = "stop")]
    Down,
    /// Mostra status de todos os containers
    Status,
    /// Controle de serviços genéricos
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
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
        /// DESTRUTIVO: Reseta todo o ambiente Podman (remove imagens, containers, volumes e cache de build)
        #[arg(long)]
        nuke: bool,
        /// Limpar tudo (padrão se nenhuma flag especificada)
        #[arg(long)]
        all: bool,
    },
    /// Gerenciamento de projetos
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Atualiza o devobox para a versão mais recente disponível no GitHub
    Update,
}

#[derive(Subcommand)]
enum ServiceAction {
    /// Inicia serviço(s)
    Start {
        /// Nome do serviço específico (opcional)
        service: Option<String>,
    },
    /// Para serviço(s)
    Stop {
        /// Nome do serviço específico (opcional)
        service: Option<String>,
    },
    /// Reinicia serviço(s)
    Restart {
        /// Nome do serviço específico (opcional)
        service: Option<String>,
    },
    /// Mostra status dos serviços
    Status,
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

#[derive(Subcommand)]
enum ProjectAction {
    /// Lista projetos disponíveis em ~/code
    List,
    /// Ativa workspace de um projeto (apenas dentro do container)
    Up {
        /// Nome do projeto
        name: String,
    },
    /// Mostra informações do contexto atual
    Info,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let default_level = if cli.verbose { "debug" } else { "info" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false) // Hide module path for cleaner CLI output by default
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(false) // Cleaner output, relies on color for level
        .init();

    match cli.command {
        None => {
            // Default behavior: open shell
            cli::runtime::shell(&cli.config_dir, cli.with_dbs, cli.auto_stop)
        }
        Some(Commands::Init { skip_cleanup }) => {
            info!(" Passo 1/2: Instalando configurações...");
            cli::setup::install(&cli.config_dir)?;

            info!("\n Passo 2/2: Construindo ambiente...");
            cli::builder::build(&cli.config_dir, skip_cleanup)?;

            info!("\n Setup completo! Use 'devobox' para abrir o shell.");
            Ok(())
        }
        Some(Commands::Install) => {
            cli::setup::install(&cli.config_dir)?;
            info!("\n Configurações instaladas em {:?}", cli.config_dir);
            info!(" Dica: Edite os arquivos e depois rode 'devobox build'");
            Ok(())
        }
        Some(Commands::Build { skip_cleanup } | Commands::Rebuild { skip_cleanup }) => {
            cli::builder::build(&cli.config_dir, skip_cleanup)
        }
        Some(Commands::Shell {
            with_dbs,
            auto_stop,
        }) => cli::runtime::shell(&cli.config_dir, with_dbs, auto_stop),
        Some(Commands::Dev { auto_stop }) => cli::runtime::shell(&cli.config_dir, true, auto_stop),
        Some(Commands::Up {
            dbs_only,
            services_only,
        }) => cli::runtime::up(&cli.config_dir, dbs_only, services_only),
        Some(Commands::Down) => cli::runtime::down(&cli.config_dir),
        Some(Commands::Status) => cli::runtime::status(&cli.config_dir),
        Some(Commands::Service { action }) => match action {
            ServiceAction::Start { service } => cli::runtime::smart_start(
                &cli.config_dir,
                service.as_deref(),
                Some(ServiceKind::Generic),
            ),
            ServiceAction::Stop { service } => cli::runtime::smart_stop(
                &cli.config_dir,
                service.as_deref(),
                Some(ServiceKind::Generic),
            ),
            ServiceAction::Restart { service } => cli::runtime::smart_restart(
                &cli.config_dir,
                service.as_deref(),
                Some(ServiceKind::Generic),
            ),
            ServiceAction::Status => cli::runtime::status(&cli.config_dir),
        },
        Some(Commands::Db { action }) => match action {
            DbAction::Start { service } => cli::runtime::smart_start(
                &cli.config_dir,
                service.as_deref(),
                Some(ServiceKind::Database),
            ),
            DbAction::Stop { service } => cli::runtime::smart_stop(
                &cli.config_dir,
                service.as_deref(),
                Some(ServiceKind::Database),
            ),
            DbAction::Restart { service } => cli::runtime::smart_restart(
                &cli.config_dir,
                service.as_deref(),
                Some(ServiceKind::Database),
            ),
            DbAction::Status => cli::runtime::status(&cli.config_dir),
        },
        Some(Commands::Cleanup {
            containers,
            images,
            volumes,
            build_cache,
            nuke,
            all,
        }) => {
            if nuke {
                return cli::runtime::nuke(&cli.config_dir);
            }

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
        Some(Commands::Project { action }) => match action {
            ProjectAction::List => cli::runtime::project_list(&cli.config_dir),
            ProjectAction::Up { name } => cli::runtime::project_up(&cli.config_dir, &name),
            ProjectAction::Info => cli::runtime::project_info(),
        },
        Some(Commands::Update) => cli::update::update(),
    }
}
