use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use devobox::infra::PodmanAdapter;
use devobox::infra::config::{databases_path, load_databases};
use devobox::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Args)]
pub struct BuilderCommand {
    #[command(subcommand)]
    pub command: BuilderAction,
}

#[derive(Subcommand)]
pub enum BuilderAction {
    /// Constrói a imagem e recria containers (devobox + bancos)
    Build {
        /// Pular limpeza automática de recursos
        #[arg(long)]
        skip_cleanup: bool,
    },
    /// Apenas verifica se as dependências para build estão disponíveis
    Check,
}

pub fn run(cmd: BuilderCommand, config_dir: &Path) -> Result<()> {
    match cmd.command {
        BuilderAction::Build { skip_cleanup } => build(config_dir, skip_cleanup),
        BuilderAction::Check => check(),
    }
}

fn check() -> Result<()> {
    println!("🔧 Checando ferramentas de build...");
    let runtime = Arc::new(PodmanAdapter::new());
    let service = ContainerService::new(runtime);

    for dep in ["podman"] {
        if service.is_command_available(dep) {
            println!("✅ {dep} disponível");
        } else {
            println!("⚠️  {dep} não encontrado no PATH");
        }
    }
    Ok(())
}

fn build(config_dir: &Path, skip_cleanup: bool) -> Result<()> {
    let runtime = Arc::new(PodmanAdapter::new());
    let container_service = Arc::new(ContainerService::new(runtime.clone()));
    let system_service = Arc::new(SystemService::new(runtime));
    let containerfile = config_dir.join("Containerfile");

    if !containerfile.exists() {
        bail!(
            "Containerfile não encontrado em {:?}. Rode 'devobox agent install' primeiro.",
            config_dir
        );
    }

    if !skip_cleanup {
        let orchestrator = Orchestrator::new(container_service.clone(), system_service.clone());
        let cleanup_options = CleanupOptions {
            containers: true,
            images: true,
            volumes: false,
            build_cache: true,
        };
        let _ = orchestrator.cleanup(&cleanup_options);
    }

    let context = config_dir.to_path_buf();
    println!("🏗️  Construindo imagem Devobox (Arch)...");
    system_service.build_image("devobox-img", &containerfile, &context)?;

    println!(
        "🗄️  Lendo bancos de dados em {:?}...",
        databases_path(config_dir)
    );
    let databases = load_databases(config_dir)?;

    if databases.is_empty() {
        println!("⚠️  Nenhum banco configurado. Pulei criação de DBs.");
    }

    for db in &databases {
        container_service.recreate(&db.to_spec())?;
    }

    let code_dir = code_mount()?;
    let dev_volumes = vec![
        code_dir.clone(),
        "devobox_mise:/home/dev/.local/share/mise".to_string(),
    ];

    let dev_spec = devobox::domain::ContainerSpec {
        name: "devobox",
        image: "devobox-img",
        ports: &[],
        env: &[],
        network: Some("host"),
        userns: Some("keep-id"),
        security_opt: Some("label=disable"),
        workdir: Some("/home/dev"),
        volumes: &dev_volumes,
        extra_args: &["-it"],
    };

    container_service.recreate(&dev_spec)?;
    println!("✅ Build concluído! Tudo pronto.");
    Ok(())
}

fn code_mount() -> Result<String> {
    let code_dir = std::env::var("DEVOBOX_CODE_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".into());
            Path::new(&home).join("code")
        });

    let code_dir = shellexpand::tilde(code_dir.to_string_lossy().as_ref()).into_owned();

    let path = PathBuf::from(&code_dir);
    if !path.exists() {
        println!(
            "⚠️  Diretório {:?} não existe. Criando para o bind mount...",
            path
        );
        std::fs::create_dir_all(&path).with_context(|| format!("criando {:?}", path))?;
    }

    Ok(format!("{}:/home/dev/code", path.to_string_lossy()))
}
