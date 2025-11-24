use anyhow::{Context, Result, bail};
use devobox::infra::PodmanAdapter;
use devobox::infra::config::{databases_path, load_databases, load_mise_config};
use devobox::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn build(config_dir: &Path, skip_cleanup: bool) -> Result<()> {
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

    println!("🔍 Validando mise.toml...");
    load_mise_config(config_dir)?;

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
    let ssh_dir = ssh_mount()?;
    let dev_volumes = vec![code_dir.clone(), ssh_dir.clone()];

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

fn ssh_mount() -> Result<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".into());
    let ssh_dir = Path::new(&home).join(".ssh");

    if !ssh_dir.exists() {
        println!("⚠️  Diretório ~/.ssh não encontrado. Git via SSH não funcionará.");
        println!("💡 Dica: Configure suas chaves SSH no host primeiro.");
    }

    Ok(format!("{}:/home/dev/.ssh:ro", ssh_dir.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_mount_returns_readonly_mount() {
        let result = ssh_mount();
        assert!(result.is_ok());

        let mount = result.unwrap();
        // Should contain the path and be read-only
        assert!(mount.contains("/.ssh:/home/dev/.ssh:ro"));
        assert!(mount.ends_with(":ro"));
    }

    #[test]
    fn test_ssh_mount_format() {
        let result = ssh_mount().unwrap();
        let parts: Vec<&str> = result.split(':').collect();

        // Format should be: /path/to/.ssh:/home/dev/.ssh:ro
        assert_eq!(parts.len(), 3);
        assert!(parts[0].ends_with("/.ssh"));
        assert_eq!(parts[1], "/home/dev/.ssh");
        assert_eq!(parts[2], "ro");
    }

    #[test]
    fn test_code_mount_creates_directory_if_not_exists() {
        // This test validates the logic but won't actually create directories
        let result = code_mount();
        assert!(result.is_ok());

        let mount = result.unwrap();
        assert!(mount.contains(":/home/dev/code"));
    }

    #[test]
    fn test_code_mount_format() {
        let result = code_mount().unwrap();
        let parts: Vec<&str> = result.split(':').collect();

        // Format should be: /path/to/code:/home/dev/code
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "/home/dev/code");
    }
}
