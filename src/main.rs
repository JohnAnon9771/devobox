mod agent;
mod builder;
mod config;
mod podman;
mod runtime;

use agent::AgentOptions;
use anyhow::Result;
use builder::BuilderCommand;
use clap::{Parser, Subcommand};
use runtime::RuntimeCommand;

#[derive(Parser)]
#[command(
    name = "devobox",
    about = "Devobox controller with split responsibilities"
)]
struct Cli {
    /// Diretório de configuração (default: ~/.config/devobox)
    #[arg(long, env = "DEVOBOX_CONFIG_DIR", default_value_t = config::default_config_dir())]
    config_dir: std::path::PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Tarefas do agente de provisionamento (checar dependências, preparar config)
    Agent(AgentOptions),
    /// Responsável por construir imagem e containers baseados em config
    Builder(BuilderCommand),
    /// Rotina de uso diário (shell, up/down, dbs)
    Runtime(RuntimeCommand),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Agent(options) => agent::run(options, &cli.config_dir),
        Commands::Builder(cmd) => builder::run(cmd, &cli.config_dir),
        Commands::Runtime(cmd) => runtime::run(cmd, &cli.config_dir),
    }
}
