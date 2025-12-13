use crate::infra::config::{default_config_dir, ensure_config_dir, install_default_config};
use anyhow::Result;
use std::path::Path;
use tracing::info;

pub fn install(config_dir: &Path) -> Result<()> {
    info!(" Preparando config em {:?}", config_dir);

    ensure_config_dir(config_dir)?;
    install_default_config(config_dir)?;

    info!(
        " Config pronto. Ajuste devobox.toml conforme necessário (padrão: {:?})",
        default_config_dir()
    );

    Ok(())
}
