use crate::config::load_databases;
use crate::podman::{PodmanCreate, build_image, create_container, remove_container};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct BuilderCommand {
    #[command(subcommand)]
    pub command: BuilderAction,
}

#[derive(Subcommand)]
pub enum BuilderAction {
    /// Constrói a imagem e recria containers (devobox + bancos)
    Build,
    /// Apenas verifica se as dependências para build estão disponíveis
    Check,
}

pub fn run(cmd: BuilderCommand, config_dir: &Path) -> Result<()> {
    match cmd.command {
        BuilderAction::Build => build(config_dir),
        BuilderAction::Check => check(),
    }
}

fn check() -> Result<()> {
    println!("🔧 Checando ferramentas de build...");
    for dep in ["podman"] {
        if crate::podman::command_available(dep) {
            println!("✅ {dep} disponível");
        } else {
            println!("⚠️  {dep} não encontrado no PATH");
        }
    }
    Ok(())
}

fn build(config_dir: &Path) -> Result<()> {
    let containerfile = config_dir.join("Containerfile");
    if !containerfile.exists() {
        bail!(
            "Containerfile não encontrado em {:?}. Rode 'devobox agent install' primeiro.",
            config_dir
        );
    }

    let context = config_dir.to_path_buf();
    println!("🏗️  Construindo imagem Devobox (Arch)...");
    build_image("devobox-img", &containerfile, &context)?;

    println!(
        "🗄️  Lendo bancos de dados em {:?}...",
        crate::config::databases_path(config_dir)
    );
    let databases = load_databases(config_dir)?;

    if databases.is_empty() {
        println!("⚠️  Nenhum banco configurado. Pulei criação de DBs.");
    }

    for db in &databases {
        let create = PodmanCreate {
            name: &db.name,
            image: &db.image,
            ports: &db.ports,
            env: &db.env,
            volumes: &db.volumes,
            network: None,
            userns: None,
            security_opt: None,
            workdir: None,
            extra_args: &[],
        };

        recreate(&create)?;
    }

    let code_dir = code_mount()?;
    let dev_volumes = vec![
        code_dir.clone(),
        "devobox_mise:/home/dev/.local/share/mise".to_string(),
    ];

    let dev = PodmanCreate {
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

    recreate(&dev)?;
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

fn recreate(spec: &PodmanCreate) -> Result<()> {
    remove_container(spec.name)?;
    create_container(spec)?;
    Ok(())
}
