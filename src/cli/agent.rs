use anyhow::Result;
use devobox::infra::config::{default_config_dir, ensure_config_dir, install_default_config};
use std::path::Path;

pub fn install(config_dir: &Path) -> Result<()> {
    println!("📁 Preparando config em {:?}", config_dir);

    ensure_config_dir(config_dir)?;
    install_default_config(config_dir)?;

    println!(
        "✅ Config pronto. Ajuste databases.yml conforme necessário (padrão: {:?})",
        default_config_dir()
    );

    Ok(())
}
