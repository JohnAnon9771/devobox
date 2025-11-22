use crate::config::{copy_template_if_missing, default_config_dir, ensure_config_dir};
use crate::podman::command_available;
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::Path;

#[derive(Args)]
pub struct AgentOptions {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Subcommand)]
pub enum AgentCommand {
    /// Verifica dependências e existência de arquivos de config
    Doctor,
    /// Copia templates de config para o diretório de configuração
    Install {
        /// Diretório fonte contendo Containerfile e databases.yml
        #[arg(long, default_value = "config")] // relativo ao repo
        source: String,
    },
}

pub fn run(command: AgentOptions, config_dir: &Path) -> Result<()> {
    match command.command {
        AgentCommand::Doctor => doctor(config_dir),
        AgentCommand::Install { source } => install(config_dir, &source),
    }
}

fn doctor(config_dir: &Path) -> Result<()> {
    println!("🔍 Checando dependências e configuração...");
    let checks = ["podman", "bash"];

    for dep in checks {
        if command_available(dep) {
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

fn install(config_dir: &Path, source: &str) -> Result<()> {
    let source_dir = Path::new(source);
    let source_dir = if source_dir.is_absolute() {
        source_dir.to_path_buf()
    } else {
        std::env::current_dir()?.join(source_dir)
    };

    println!(
        "📁 Preparando config em {:?} (templates de {:?})",
        config_dir, source_dir
    );

    ensure_config_dir(config_dir)?;
    copy_template_if_missing(&source_dir, config_dir)?;

    println!(
        "✅ Config pronto. Ajuste databases.yml conforme necessário (padrão: {:?})",
        default_config_dir()
    );

    Ok(())
}
