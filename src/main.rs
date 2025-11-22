mod cli;

use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::{AgentOptions, BuilderCommand, RuntimeCommand};

#[derive(Parser)]
#[command(
    name = "devobox",
    about = "Devobox controller with split responsibilities"
)]
struct Cli {
    /// Diretório de configuração (default: ~/.config/devobox)
    #[arg(long, env, default_value_os_t = devobox::infra::config::default_config_dir())]
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
        Commands::Agent(options) => cli::agent::run(options, &cli.config_dir),
        Commands::Builder(cmd) => cli::builder::run(cmd, &cli.config_dir),
        Commands::Runtime(cmd) => cli::runtime::run(cmd, &cli.config_dir),
    }
}
