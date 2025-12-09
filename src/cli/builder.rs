use crate::infra::PodmanAdapter;
use crate::infra::config::{load_app_config, load_mise_config};
use crate::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use anyhow::{Context, Result, bail};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tracing::{debug, info, warn};

const CONTAINER_SSH_SOCK_PATH: &str = "/run/host-services/ssh-auth.sock";
const PERSISTENT_MISE_SHARE_PATH: &str = "/home/dev/.local/share/mise";
const PERSISTENT_MISE_CONFIG_PATH: &str = "/home/dev/.config/mise";
const PERSISTENT_CARGO_PATH: &str = "/home/dev/.cargo";
const PERSISTENT_NVIM_SHARE_PATH: &str = "/home/dev/.local/share/nvim";
const PERSISTENT_NVIM_STATE_PATH: &str = "/home/dev/.local/state/nvim";
const PERSISTENT_BASH_HISTORY_PATH: &str = "/home/dev/.local/state/bash";

/// Detects Podman socket strictly following Linux standards
/// Priority: Env Var -> XDG Rootless -> UID Rootless -> System Rootful
fn detect_podman_socket() -> Option<PathBuf> {
    if let Ok(sock) = std::env::var("PODMAN_SOCK") {
        let path = PathBuf::from(sock);
        if path.exists()
            && std::fs::metadata(&path)
                .map(|m| m.file_type().is_socket())
                .unwrap_or(false)
        {
            return Some(path);
        }
    }

    let uid = std::fs::metadata("/proc/self").map(|m| m.uid()).ok()?;

    let candidates = vec![
        std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .map(|dir| PathBuf::from(dir).join("podman/podman.sock")),
        Some(PathBuf::from(format!(
            "/run/user/{}/podman/podman.sock",
            uid
        ))),
        Some(PathBuf::from("/run/podman/podman.sock")),
    ];

    candidates.into_iter().flatten().find(|path| {
        std::fs::metadata(path)
            .map(|m| m.file_type().is_socket())
            .unwrap_or(false)
    })
}

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

struct SshFeature;
impl HostFeature for SshFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
        let mut config = ContainerConfigFragment::default();

        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".into());
        let ssh_dir = Path::new(&home).join(".ssh");

        if !ssh_dir.exists() {
            warn!("  Diretório ~/.ssh não encontrado. Git via SSH não funcionará.");
            info!(" Dica: Configure suas chaves SSH no host primeiro.");
        }
        config
            .volumes
            .push(format!("{}:/home/dev/.ssh:ro", ssh_dir.to_string_lossy()));

        if let Ok(auth_sock) = std::env::var("SSH_AUTH_SOCK") {
            let auth_path = PathBuf::from(&auth_sock);
            if auth_path.exists()
                && std::fs::metadata(&auth_path)
                    .map(|m| m.file_type().is_socket())
                    .unwrap_or(false)
            {
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
            } else {
                warn!(
                    "  SSH_AUTH_SOCK aponta para um caminho inexistente ou não-socket: {:?}",
                    auth_path
                );
            }
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
        if path.exists()
            && std::fs::metadata(&path)
                .map(|m| m.file_type().is_socket())
                .unwrap_or(false)
        {
            info!(" GPG Agent detectado: {}", socket_path);
            Ok(Some(ContainerConfigFragment {
                volumes: vec![format!("{}:/home/dev/.gnupg/S.gpg-agent", socket_path)],
                ..Default::default()
            }))
        } else {
            debug!(
                "  GPG Agent detectado em {:?}, mas não é um socket ou não existe.",
                path
            );
            Ok(None)
        }
    }
}

struct PodmanFeature;
impl HostFeature for PodmanFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
        // 1. Prevent "Inception" (Devobox inside Devobox)
        if std::env::var("DEVOBOX_CONTAINER").is_ok() {
            debug!("  Detectado ambiente containerizado: pulando montagem do socket Podman.");
            return Ok(None);
        }

        // 2. Detect Socket
        let socket_path = match detect_podman_socket() {
            Some(path) => path,
            None => {
                debug!("  Socket Podman não encontrado (Rootless ou Rootful).");
                return Ok(None);
            }
        };

        // 3. Check Permissions (Crucial for Linux)
        // If we can't read the parent directory, we might not be able to mount the socket.
        // `socket_path.parent()` can be None if it's a root path, handle that case.
        if socket_path
            .parent()
            .is_none_or(|parent| std::fs::read_dir(parent).is_err())
        {
            warn!(
                "  Socket Podman encontrado em {:?}, mas sem permissão de acesso ao diretório pai. Verifique as permissões.",
                socket_path
            );
            return Ok(None);
        }

        info!(
            "  Socket Podman detectado: {:?} (controle de containers do host habilitado)",
            socket_path
        );

        // Standard path inside the container
        let container_socket_path = "/run/podman/podman.sock";

        Ok(Some(ContainerConfigFragment {
            volumes: vec![format!(
                "{}:{}",
                socket_path.to_string_lossy(),
                container_socket_path
            )],
            env: vec![format!("PODMAN_SOCK={}", container_socket_path)],
            ..Default::default()
        }))
    }
}

struct GuiFeature;
impl HostFeature for GuiFeature {
    fn configure(&self, _ctx: &BuildContext) -> Result<Option<ContainerConfigFragment>> {
        let mut config = ContainerConfigFragment::default();

        // Wayland
        if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY")
            && let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR")
        {
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

pub fn build(config_dir: &Path, skip_cleanup: bool) -> Result<()> {
    let app_config = load_app_config(config_dir)?;

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

    info!("  Construindo imagem {} (Arch)...", image_name);
    system_service.build_image(&image_name, &containerfile, &context)?;

    info!(" Validando mise.toml...");
    let mise_toml_path = config_dir.join(
        app_config
            .paths
            .mise_toml
            .clone()
            .context("mise.toml path not set in config")?,
    );
    load_mise_config(&mise_toml_path)?;

    info!(" Resolvendo serviços (incluindo dependências)...");
    let services = crate::infra::config::resolve_all_services(config_dir, &app_config)?;

    if services.is_empty() {
        warn!("  Nenhum serviço configurado. Pulei criação de serviços.");
    }

    for svc in &services {
        container_service.recreate(&svc.to_spec())?;
    }

    let features: Vec<Box<dyn HostFeature>> = vec![
        Box::new(CodeMountFeature),
        Box::new(SshFeature),
        Box::new(GpgFeature),
        Box::new(PodmanFeature),
        Box::new(GuiFeature),
        Box::new(PersistenceFeature),
    ];

    let build_ctx = BuildContext {};
    let mut final_config = ContainerConfigFragment::default();

    for feature in features {
        if let Ok(Some(fragment)) = feature.configure(&build_ctx) {
            final_config = final_config.merge(fragment);
        }
    }

    let main_container_name = app_config
        .container
        .name
        .context("Main container name not set in config")?;
    let main_container_workdir = app_config
        .container
        .workdir
        .context("Main container workdir not set in config")?;

    let mut all_extra_args = vec!["-it".to_string()];
    all_extra_args.extend(final_config.extra_args);
    all_extra_args.extend(final_config.devices);

    let mut container_env = final_config.env.clone();
    container_env.push("DEVOBOX_CONTAINER=1".to_string());

    let extra_args_refs: Vec<&str> = all_extra_args.iter().map(|s| s.as_str()).collect();

    let dev_spec = crate::domain::ContainerSpec {
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
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env_vars<F>(vars: Vec<(&str, Option<&str>)>, test: F)
    where
        F: FnOnce(),
    {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut original_vars = Vec::new();

        for (key, val) in &vars {
            original_vars.push((key, std::env::var(key)));
            match val {
                Some(v) => unsafe { std::env::set_var(key, v) },
                None => unsafe { std::env::remove_var(key) },
            }
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));

        for (key, val) in original_vars {
            match val {
                Ok(v) => unsafe { std::env::set_var(key, v) },
                Err(_) => unsafe { std::env::remove_var(key) },
            }
        }

        if let Err(err) = result {
            std::panic::resume_unwind(err);
        }
    }

    #[test]
    fn test_config_fragment_merge() {
        let f1 = ContainerConfigFragment {
            volumes: vec!["v1".into()],
            env: vec!["e1".into()],
            devices: vec!["d1".into()],
            extra_args: vec!["a1".into()],
        };
        let f2 = ContainerConfigFragment {
            volumes: vec!["v2".into()],
            env: vec!["e2".into()],
            devices: vec!["d2".into()],
            extra_args: vec!["a2".into()],
        };

        let merged = f1.merge(f2);
        assert_eq!(merged.volumes, vec!["v1", "v2"]);
        assert_eq!(merged.env, vec!["e1", "e2"]);
        assert_eq!(merged.devices, vec!["d1", "d2"]);
        assert_eq!(merged.extra_args, vec!["a1", "a2"]);
    }

    #[test]
    fn test_podman_feature_inception_prevention() {
        with_env_vars(vec![("DEVOBOX_CONTAINER", Some("1"))], || {
            let feature = PodmanFeature;
            let ctx = BuildContext {};
            let result = feature.configure(&ctx).unwrap();

            assert!(
                result.is_none(),
                "PodmanFeature deve retornar None se DEVOBOX_CONTAINER estiver definido"
            );
        });
    }

    #[test]
    fn test_podman_feature_no_socket() {
        with_env_vars(
            vec![
                ("DEVOBOX_CONTAINER", None),
                ("PODMAN_SOCK", None),
                ("XDG_RUNTIME_DIR", None),
            ],
            || {
                let feature = PodmanFeature;
                let ctx = BuildContext {};
                let result = feature.configure(&ctx).unwrap();
                assert!(
                    result.is_none(),
                    "Sem socket detectável, deve retornar None"
                );
            },
        );
    }

    #[test]
    fn test_ssh_feature_basic_config() {
        with_env_vars(
            vec![("SSH_AUTH_SOCK", None), ("HOME", Some("/tmp"))],
            || {
                let feature = SshFeature;
                let ctx = BuildContext {};
                let res = feature.configure(&ctx).unwrap();

                assert!(res.is_some());
                let config = res.unwrap();

                assert!(
                    config
                        .volumes
                        .iter()
                        .any(|v| v.contains(":/home/dev/.ssh:ro")),
                    "Deve montar ~/.ssh como read-only"
                );
                assert!(config.env.is_empty());
            },
        );
    }

    #[test]
    fn test_persistence_feature_volumes() {
        let feature = PersistenceFeature;
        let ctx = BuildContext {};
        let config = feature.configure(&ctx).unwrap().unwrap();

        let required_volumes = vec![
            "devobox_data_mise",
            "devobox_data_cargo",
            "devobox_data_bash_history",
            "devobox_data_nvim_state",
        ];

        for vol in required_volumes {
            assert!(
                config.volumes.iter().any(|v| v.contains(vol)),
                "Configuração de persistência deve conter volume '{}'",
                vol
            );
        }
    }

    #[test]
    fn test_codemount_feature() {
        with_env_vars(
            vec![("DEVOBOX_CODE_DIR", Some("/tmp/my-code-project"))],
            || {
                let feature = CodeMountFeature;
                let ctx = BuildContext {};
                std::fs::create_dir_all("/tmp/my-code-project").ok();

                let res = feature.configure(&ctx).unwrap();
                assert!(res.is_some());
                let config = res.unwrap();

                assert!(
                    config
                        .volumes
                        .iter()
                        .any(|v| v.starts_with("/tmp/my-code-project:")),
                    "Deve montar o diretório de código especificado via ENV"
                );

                std::fs::remove_dir_all("/tmp/my-code-project").ok();
            },
        );
    }
}
