use anyhow::{Context, Result, bail};
use devobox::infra::PodmanAdapter;
use devobox::infra::config::{load_app_config, load_mise_config};
use devobox::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

const CONTAINER_SSH_SOCK_PATH: &str = "/run/host-services/ssh-auth.sock";

const PERSISTENT_MISE_SHARE_PATH: &str = "/home/dev/.local/share/mise";
const PERSISTENT_MISE_CONFIG_PATH: &str = "/home/dev/.config/mise";
const PERSISTENT_CARGO_PATH: &str = "/home/dev/.cargo";
const PERSISTENT_NVIM_SHARE_PATH: &str = "/home/dev/.local/share/nvim";
const PERSISTENT_NVIM_STATE_PATH: &str = "/home/dev/.local/state/nvim";
const PERSISTENT_BASH_HISTORY_PATH: &str = "/home/dev/.local/state/bash";

pub fn build(config_dir: &Path, skip_cleanup: bool) -> Result<()> {
    let app_config = load_app_config(config_dir)?; // Load the merged config

    let runtime = Arc::new(PodmanAdapter::new());
    let container_service = Arc::new(ContainerService::new(runtime.clone()));
    let system_service = Arc::new(SystemService::new(runtime));
    let containerfile_path_from_config = app_config
        .paths
        .containerfile
        .clone()
        .context("Containerfile path not set in config")?;
    let containerfile = config_dir.join(containerfile_path_from_config);

    if !containerfile.exists() {
        bail!(
            "Containerfile não encontrado em {:?}. Rode 'devobox setup install' primeiro.",
            config_dir
        );
    }

    if !skip_cleanup {
        let orchestrator = Orchestrator::new(container_service.clone(), system_service.clone());
        let cleanup_options = CleanupOptions {
            containers: true,
            images: true,
            volumes: false,
            build_cache: false,
        };
        let _ = orchestrator.cleanup(&cleanup_options);
    }

    let context = config_dir.to_path_buf();
    let image_name = app_config
        .build
        .image_name
        .clone()
        .context("Image name not set in config")?;
    println!("🏗️  Construindo imagem {} (Arch)...", image_name);
    system_service.build_image(&image_name, &containerfile, &context)?;

    println!("🔍 Validando mise.toml...");
    let mise_toml_path = config_dir.join(
        app_config
            .paths
            .mise_toml
            .clone()
            .context("mise.toml path not set in config")?,
    );
    load_mise_config(&mise_toml_path)?;

    println!("🗄️  Resolvendo serviços (incluindo dependências)...");
    let services = devobox::infra::config::resolve_all_services(config_dir, &app_config)?;

    if services.is_empty() {
        println!("⚠️  Nenhum serviço configurado. Pulei criação de serviços.");
    }

    for svc in &services {
        container_service.recreate(&svc.to_spec())?;
    }

    let code_mount_str = code_mount()?;
    let ssh_mount_str = ssh_mount()?;
    let mut dev_volumes = vec![code_mount_str, ssh_mount_str];
    let mut dev_env = vec![];

    if let Ok(auth_sock) = std::env::var("SSH_AUTH_SOCK") {
        let auth_path = PathBuf::from(&auth_sock);
        dev_volumes.push(format!(
            "{}:{}",
            auth_path.to_string_lossy(),
            CONTAINER_SSH_SOCK_PATH
        ));
        dev_env.push(format!("SSH_AUTH_SOCK={}", CONTAINER_SSH_SOCK_PATH));
        println!(
            "🔑 SSH Agent (`{}`) detectado e configurado para o Hub.",
            auth_sock
        );
    }

    if let Ok(Some(gpg_mount)) = get_gpg_mount() {
        dev_volumes.push(gpg_mount);
    }

    let (gui_volumes, gui_envs, gui_args) = get_gui_support();
    dev_volumes.extend(gui_volumes);
    dev_env.extend(gui_envs);

    dev_volumes.extend(get_persistent_volumes());

    let main_container_name = app_config
        .container
        .name
        .context("Main container name not set in config")?;
    let main_container_workdir = app_config
        .container
        .workdir
        .context("Main container workdir not set in config")?;

    let mut extra_args_storage = Vec::new();
    extra_args_storage.push("-it".to_string());
    extra_args_storage.extend(gui_args);

    let extra_args_refs: Vec<&str> = extra_args_storage.iter().map(|s| s.as_str()).collect();

    let dev_spec = devobox::domain::ContainerSpec {
        name: &main_container_name,
        image: &image_name,
        ports: &[],
        env: &dev_env,
        network: Some("host"),
        userns: Some("keep-id"),
        security_opt: Some("label=disable"),
        workdir: Some(
            main_container_workdir
                .to_str()
                .context("Container workdir is not valid UTF-8")?,
        ),
        volumes: &dev_volumes,
        extra_args: &extra_args_refs,
        healthcheck_command: None,
        healthcheck_interval: None,
        healthcheck_timeout: None,
        healthcheck_retries: None,
    };

    container_service.recreate(&dev_spec)?;
    println!("✅ Build concluído! Tudo pronto.");
    Ok(())
}

fn get_gpg_mount() -> Result<Option<String>> {
    // Check if gpgconf is available
    if Command::new("which")
        .arg("gpgconf")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        return Ok(None);
    }

    let output = Command::new("gpgconf")
        .args(["--list-dirs", "agent-socket"])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let socket_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if socket_path.is_empty() {
        return Ok(None);
    }

    let path = PathBuf::from(&socket_path);
    if !path.exists() {
        return Ok(None);
    }

    println!("🔐 GPG Agent detectado: {}", socket_path);
    // Mount to standard location in container: /home/dev/.gnupg/S.gpg-agent
    Ok(Some(format!(
        "{}:/home/dev/.gnupg/S.gpg-agent",
        socket_path
    )))
}

fn get_gui_support() -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut volumes = Vec::new();
    let mut envs = Vec::new();
    let mut devices = Vec::new();

    // Wayland
    if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") {
        if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
            let host_socket = Path::new(&xdg_runtime).join(&wayland_display);
            if host_socket.exists() {
                println!("🖼️  Wayland detectado: {}", wayland_display);
                let container_socket = format!("/run/user/1000/{}", wayland_display);
                volumes.push(format!(
                    "{}:{}",
                    host_socket.to_string_lossy(),
                    container_socket
                ));
                envs.push(format!("WAYLAND_DISPLAY={}", wayland_display));
                envs.push("XDG_RUNTIME_DIR=/run/user/1000".to_string());
            }
        }
    }

    // X11
    if let Ok(display) = std::env::var("DISPLAY") {
        let x11_socket_dir = Path::new("/tmp/.X11-unix");
        if x11_socket_dir.exists() {
            println!("🖼️  X11 detectado: {}", display);
            volumes.push(format!("{}:/tmp/.X11-unix:ro", x11_socket_dir.to_string_lossy()));
            envs.push(format!("DISPLAY={}", display));
        }
    }

    // GPU / DRI
    if Path::new("/dev/dri").exists() {
        println!("🎮 GPU aceleração detectada (/dev/dri)");
        devices.push("--device".to_string());
        devices.push("/dev/dri".to_string());
    }

    // Fonts
    let font_dirs = vec![
        "/usr/share/fonts",
        "/usr/local/share/fonts",
    ];

    for (i, dir) in font_dirs.iter().enumerate() {
        let p = Path::new(dir);
        if p.exists() {
            volumes.push(format!(
                "{}:/home/dev/.local/share/fonts/host_{}:ro",
                p.to_string_lossy(),
                i
            ));
        }
    }

    // User fonts
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        let user_fonts_linux = Path::new(&home).join(".local/share/fonts");
        if user_fonts_linux.exists() {
            volumes.push(format!(
                "{}:/home/dev/.local/share/fonts/host_user:ro",
                user_fonts_linux.to_string_lossy()
            ));
        }
    }

    (volumes, envs, devices)
}

fn get_persistent_volumes() -> Vec<String> {
    vec![
        format!("devobox_data_mise:{}", PERSISTENT_MISE_SHARE_PATH),
        format!("devobox_data_mise_config:{}", PERSISTENT_MISE_CONFIG_PATH),
        format!("devobox_data_cargo:{}", PERSISTENT_CARGO_PATH),
        format!("devobox_data_nvim_share:{}", PERSISTENT_NVIM_SHARE_PATH),
        format!("devobox_data_nvim_state:{}", PERSISTENT_NVIM_STATE_PATH),
        format!("devobox_data_bash_history:{}", PERSISTENT_BASH_HISTORY_PATH),
    ]
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
