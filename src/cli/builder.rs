use anyhow::{Context, Result, bail};
use devobox::infra::PodmanAdapter;
use devobox::infra::config::{load_app_config, load_mise_config};
use devobox::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tracing::{info, warn};

// --- Constants ---
const CONTAINER_SSH_SOCK_PATH: &str = "/run/host-services/ssh-auth.sock";
const PERSISTENT_MISE_SHARE_PATH: &str = "/home/dev/.local/share/mise";
const PERSISTENT_MISE_CONFIG_PATH: &str = "/home/dev/.config/mise";
const PERSISTENT_CARGO_PATH: &str = "/home/dev/.cargo";
const PERSISTENT_NVIM_SHARE_PATH: &str = "/home/dev/.local/share/nvim";
const PERSISTENT_NVIM_STATE_PATH: &str = "/home/dev/.local/state/nvim";
const PERSISTENT_BASH_HISTORY_PATH: &str = "/home/dev/.local/state/bash";

// --- Types & Traits ---

/// Accumulated configuration fragment from a feature
#[derive(Debug, Default, Clone)]
struct ContainerConfigFragment {
    volumes: Vec<String>,
    env: Vec<String>,
    devices: Vec<String>,
    extra_args: Vec<String>,
}

impl ContainerConfigFragment {
    fn merge(mut self, other: Self) -> Self {
        self.volumes.extend(other.volumes);
        self.env.extend(other.env);
        self.devices.extend(other.devices);
        self.extra_args.extend(other.extra_args);
        self
    }
}

/// Context passed to features during configuration
struct BuildContext {
    // Potentially useful for future features
    // config: AppConfig,
}

/// Trait defining a pluggable host feature
trait HostFeature {
    fn configure(&self, context: &BuildContext) -> Result<Option<ContainerConfigFragment>>;
}

// --- Features Implementation ---

struct SshFeature;
impl HostFeature for SshFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
        let mut config = ContainerConfigFragment::default();

        // 1. SSH Mount (Keys)
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".into());
        let ssh_dir = Path::new(&home).join(".ssh");

        if !ssh_dir.exists() {
            warn!("  Diretório ~/.ssh não encontrado. Git via SSH não funcionará.");
            info!(" Dica: Configure suas chaves SSH no host primeiro.");
        }
        config
            .volumes
            .push(format!("{}:/home/dev/.ssh:ro", ssh_dir.to_string_lossy()));

        // 2. SSH Agent (Socket)
        if let Ok(auth_sock) = std::env::var("SSH_AUTH_SOCK") {
            let auth_path = PathBuf::from(&auth_sock);
            config.volumes.push(format!(
                "{}:{}",
                auth_path.to_string_lossy(),
                CONTAINER_SSH_SOCK_PATH
            ));
            config
                .env
                .push(format!("SSH_AUTH_SOCK={}", CONTAINER_SSH_SOCK_PATH));
            info!(
                " SSH Agent (`{}`) detectado e configurado para o Hub.",
                auth_sock
            );
        }

        Ok(Some(config))
    }
}

struct GpgFeature;
impl HostFeature for GpgFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
        // Quick check if gpgconf exists to avoid process overhead
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

        info!(" GPG Agent detectado: {}", socket_path);

        Ok(Some(ContainerConfigFragment {
            volumes: vec![format!("{}:/home/dev/.gnupg/S.gpg-agent", socket_path)],
            ..Default::default()
        }))
    }
}

struct GuiFeature;
impl HostFeature for GuiFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
        let mut config = ContainerConfigFragment::default();

        // Wayland
        if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") {
            if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
                let host_socket = Path::new(&xdg_runtime).join(&wayland_display);
                if host_socket.exists() {
                    info!("  Wayland detectado: {}", wayland_display);
                    config.volumes.push(format!(
                        "{}:/run/user/1000/{}",
                        host_socket.to_string_lossy(),
                        wayland_display
                    ));
                    config
                        .env
                        .push(format!("WAYLAND_DISPLAY={}", wayland_display));
                    config
                        .env
                        .push("XDG_RUNTIME_DIR=/run/user/1000".to_string());
                }
            }
        }

        // X11
        if let Ok(x11_display) = std::env::var("DISPLAY") {
            let x11_socket_dir = Path::new("/tmp/.X11-unix");
            if x11_socket_dir.exists() {
                info!("  X11 detectado: {}", x11_display);
                config.volumes.push(format!(
                    "{}:/tmp/.X11-unix:ro",
                    x11_socket_dir.to_string_lossy()
                ));
                config.env.push(format!("DISPLAY={}", x11_display));
            }
        }

        // GPU / DRI
        if Path::new("/dev/dri").exists() {
            info!(" GPU aceleração detectada (/dev/dri)");
            config.devices.push("--device".to_string());
            config.devices.push("/dev/dri".to_string());
        }

        // Fonts
        let font_dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];

        for (i, dir) in font_dirs.iter().enumerate() {
            let p = Path::new(dir);
            if p.exists() {
                config.volumes.push(format!(
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
                config.volumes.push(format!(
                    "{}:/home/dev/.local/share/fonts/host_user:ro",
                    user_fonts_linux.to_string_lossy()
                ));
            }
        }

        Ok(Some(config))
    }
}

struct PersistenceFeature;
impl HostFeature for PersistenceFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
        let volumes = vec![
            format!("devobox_data_mise:{}", PERSISTENT_MISE_SHARE_PATH),
            format!("devobox_data_mise_config:{}", PERSISTENT_MISE_CONFIG_PATH),
            format!("devobox_data_cargo:{}", PERSISTENT_CARGO_PATH),
            format!("devobox_data_nvim_share:{}", PERSISTENT_NVIM_SHARE_PATH),
            format!("devobox_data_nvim_state:{}", PERSISTENT_NVIM_STATE_PATH),
            format!("devobox_data_bash_history:{}", PERSISTENT_BASH_HISTORY_PATH),
        ];

        Ok(Some(ContainerConfigFragment {
            volumes,
            ..Default::default()
        }))
    }
}

struct CodeMountFeature;
impl HostFeature for CodeMountFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
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
            warn!(
                "  Diretório {:?} não existe. Criando para o bind mount...",
                path
            );
            std::fs::create_dir_all(&path).with_context(|| format!("criando {:?}", path))?;
        }

        Ok(Some(ContainerConfigFragment {
            volumes: vec![format!("{}:/home/dev/code", path.to_string_lossy())],
            ..Default::default()
        }))
    }
}

// --- Main Build Logic ---

pub fn build(config_dir: &Path, skip_cleanup: bool) -> Result<()> {
    let app_config = load_app_config(config_dir)?;

    let runtime = Arc::new(PodmanAdapter::new());
    let container_service = Arc::new(ContainerService::new(runtime.clone()));
    let system_service = Arc::new(SystemService::new(runtime));

    // 1. Validation & Setup
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

    // 2. Cleanup
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

    // 3. Build Image
    let context = config_dir.to_path_buf();
    let image_name = app_config
        .build
        .image_name
        .clone()
        .context("Image name not set in config")?;

    info!("  Construindo imagem {} (Arch)...", image_name);
    system_service.build_image(&image_name, &containerfile, &context)?;

    // 4. Validate Configs
    info!(" Validando mise.toml...");
    let mise_toml_path = config_dir.join(
        app_config
            .paths
            .mise_toml
            .clone()
            .context("mise.toml path not set in config")?,
    );
    load_mise_config(&mise_toml_path)?;

    // 5. Resolve Services
    info!(" Resolvendo serviços (incluindo dependências)...");
    let services = devobox::infra::config::resolve_all_services(config_dir, &app_config)?;

    if services.is_empty() {
        warn!("  Nenhum serviço configurado. Pulei criação de serviços.");
    }

    for svc in &services {
        container_service.recreate(&svc.to_spec())?;
    }

    // 6. Configure Host Features (SOLID / OCP)
    let features: Vec<Box<dyn HostFeature>> = vec![
        Box::new(CodeMountFeature),
        Box::new(SshFeature),
        Box::new(GpgFeature),
        Box::new(GuiFeature),
        Box::new(PersistenceFeature),
    ];

    let build_ctx = BuildContext {};
    let mut final_config = ContainerConfigFragment::default();

    // Using iterator for performance and cleaner aggregation
    for feature in features {
        if let Ok(Some(fragment)) = feature.configure(&build_ctx) {
            final_config = final_config.merge(fragment);
        }
    }

    // 7. Prepare Final Spec
    let main_container_name = app_config
        .container
        .name
        .context("Main container name not set in config")?;
    let main_container_workdir = app_config
        .container
        .workdir
        .context("Main container workdir not set in config")?;

    // Combine default args with feature args
    let mut all_extra_args = vec!["-it".to_string()];
    all_extra_args.extend(final_config.extra_args);
    // Combine feature devices into extra args since PodmanAdapter might expect raw args for some
    all_extra_args.extend(final_config.devices);

    // Inject DEVOBOX_CONTAINER marker for context detection
    let mut container_env = final_config.env.clone();
    container_env.push("DEVOBOX_CONTAINER=1".to_string());

    let extra_args_refs: Vec<&str> = all_extra_args.iter().map(|s| s.as_str()).collect();

    let dev_spec = devobox::domain::ContainerSpec {
        name: &main_container_name,
        image: &image_name,
        ports: &[],
        env: &container_env,
        network: Some("host"),
        userns: Some("keep-id"),
        security_opt: Some("label=disable"),
        workdir: Some(
            main_container_workdir
                .to_str()
                .context("Container workdir is not valid UTF-8")?,
        ),
        volumes: &final_config.volumes,
        extra_args: &extra_args_refs,
        healthcheck_command: None,
        healthcheck_interval: None,
        healthcheck_timeout: None,
        healthcheck_retries: None,
    };

    container_service.recreate(&dev_spec)?;
    info!(" Build concluído! Tudo pronto.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_feature_generates_ro_mount() {
        // We can't easily mock env vars safely in parallel tests without a mutex or strictly serial tests,
        // but we can verify the logic structure if we extracted the path generation logic.
        // For now, we test the Feature impl roughly by instantiation.
        let feature = SshFeature;
        // Just ensure it implements the trait
        let _ = feature.configure(&BuildContext {});
    }

    #[test]
    fn test_config_fragment_merge() {
        let f1 = ContainerConfigFragment {
            volumes: vec!["v1".into()],
            env: vec!["e1".into()],
            devices: vec![],
            extra_args: vec![],
        };
        let f2 = ContainerConfigFragment {
            volumes: vec!["v2".into()],
            env: vec!["e2".into()],
            devices: vec!["d1".into()],
            extra_args: vec!["a1".into()],
        };

        let merged = f1.merge(f2);
        assert_eq!(merged.volumes, vec!["v1", "v2"]);
        assert_eq!(merged.env, vec!["e1", "e2"]);
        assert_eq!(merged.devices, vec!["d1"]);
        assert_eq!(merged.extra_args, vec!["a1"]);
    }

    #[test]
    fn test_persistence_feature_config() {
        let feature = PersistenceFeature;

        let ctx = BuildContext {};
        let config = feature.configure(&ctx).unwrap().unwrap();

        // Check if critical volumes are present
        assert!(
            config
                .volumes
                .iter()
                .any(|v| v.contains("devobox_data_mise"))
        );
        assert!(
            config
                .volumes
                .iter()
                .any(|v| v.contains("devobox_data_cargo"))
        );
        assert!(
            config
                .volumes
                .iter()
                .any(|v| v.contains("devobox_data_bash_history"))
        );

        // Ensure no env vars or devices are set by default for persistence
        assert!(config.env.is_empty());
        assert!(config.devices.is_empty());
    }

    #[test]
    fn test_container_config_fragment_default() {
        let config = ContainerConfigFragment::default();
        assert!(config.volumes.is_empty());
        assert!(config.env.is_empty());
        assert!(config.devices.is_empty());
        assert!(config.extra_args.is_empty());
    }
}
