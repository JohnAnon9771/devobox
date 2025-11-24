use anyhow::Result;
use clap::{Args, Subcommand};
use devobox::infra::PodmanAdapter;
use devobox::infra::config::{default_config_dir, ensure_config_dir, install_default_config};
use devobox::services::ContainerService;
use std::path::Path;
use std::sync::Arc;

#[derive(Args)]
pub struct AgentOptions {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Subcommand)]
pub enum AgentCommand {
    /// Verifica dependências e existência de arquivos de config
    Doctor,
    /// Instala templates de config padrão para o diretório de configuração
    Install,
}

pub fn run(command: AgentOptions, config_dir: &Path) -> Result<()> {
    match command.command {
        AgentCommand::Doctor => doctor(config_dir),
        AgentCommand::Install => install(config_dir),
    }
}

fn doctor(config_dir: &Path) -> Result<()> {
    println!("🔍 Checando dependências e configuração...");
    let checks = ["podman", "bash"];
    let runtime = Arc::new(PodmanAdapter::new());
    let service = ContainerService::new(runtime);

    for dep in checks {
        if service.is_command_available(dep) {
            println!("✅ {dep} disponível");
        } else {
            println!("⚠️  {dep} não encontrado no PATH");
        }
    }

    if config_dir.exists() {
        println!("✅ Diretório de config: {:?}", config_dir);
    } else {
        println!(
            "⚠️  Diretório de config ausente em {:?} (use agent install)",
            config_dir
        );
    }

    Ok(())
}

fn install(config_dir: &Path) -> Result<()> {
    println!("📁 Preparando config em {:?}", config_dir);

    ensure_config_dir(config_dir)?;
    install_default_config(config_dir)?;

    println!(
        "✅ Config pronto. Ajuste databases.yml conforme necessário (padrão: {:?})",
        default_config_dir()
    );

    Ok(())
}
