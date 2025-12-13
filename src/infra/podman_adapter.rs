use crate::domain::traits::ContainerHealthStatus;
use crate::domain::{Container, ContainerRuntime, ContainerSpec, ContainerState};
use anyhow::{Context, Result, bail};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct PodmanAdapter;

impl PodmanAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PodmanAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerRuntime for PodmanAdapter {
    fn get_container(&self, name: &str) -> Result<Container> {
        let state = get_container_state(name)?;
        Ok(Container::new(name.to_string(), state))
    }

    fn get_container_health(&self, name: &str) -> Result<ContainerHealthStatus> {
        let output = Command::new("podman")
            .args(["inspect", name, "--format", "{{.State.Health.Status}}"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("checando health de {name}"))?;

        if !output.status.success() {
            // If inspect fails (e.g., container not found), treat as Unknown
            return Ok(ContainerHealthStatus::Unknown);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        match stdout.as_str() {
            "healthy" => Ok(ContainerHealthStatus::Healthy),
            "unhealthy" => Ok(ContainerHealthStatus::Unhealthy),
            "starting" => Ok(ContainerHealthStatus::Starting),
            "" => {
                // Check if container exists and running. If it exists but has no healthcheck, it's NotApplicable
                let state = get_container_state(name)?;
                match state {
                    ContainerState::Running | ContainerState::Stopped => {
                        Ok(ContainerHealthStatus::NotApplicable)
                    }
                    _ => Ok(ContainerHealthStatus::Unknown), // Container not created, etc.
                }
            }
            _ => Ok(ContainerHealthStatus::Unknown),
        }
    }

    fn start_container(&self, name: &str) -> Result<()> {
        podman(
            ["start", name],
            &format!("iniciando container {name}"),
            true,
        )
    }

    fn stop_container(&self, name: &str) -> Result<()> {
        podman(["stop", name], &format!("parando container {name}"), true)
    }

    fn create_container(&self, spec: &ContainerSpec) -> Result<()> {
        let mut args: Vec<String> = vec!["create".into(), "--name".into(), spec.name.into()];

        if let Some(net) = spec.network {
            args.push("--network".into());
            args.push(net.into());
        }
        if let Some(userns) = spec.userns {
            args.push("--userns".into());
            args.push(userns.into());
        }
        if let Some(sec) = spec.security_opt {
            args.push("--security-opt".into());
            args.push(sec.into());
        }
        if let Some(wd) = spec.workdir {
            args.push("-w".into());
            args.push(wd.into());
        }

        for port in spec.ports {
            args.push("-p".into());
            args.push(port.clone());
        }

        for env in spec.env {
            args.push("-e".into());
            args.push(env.clone());
        }

        for volume in spec.volumes {
            args.push("-v".into());
            args.push(volume.clone());
        }

        if let Some(hc_cmd) = spec.healthcheck_command {
            args.push("--healthcheck-cmd".into());
            args.push(hc_cmd.into());
        }
        if let Some(hc_interval) = spec.healthcheck_interval {
            args.push("--healthcheck-interval".into());
            args.push(hc_interval.into());
        }
        if let Some(hc_timeout) = spec.healthcheck_timeout {
            args.push("--healthcheck-timeout".into());
            args.push(hc_timeout.into());
        }
        if let Some(hc_retries) = spec.healthcheck_retries {
            args.push("--healthcheck-retries".into());
            args.push(hc_retries.to_string());
        }

        for extra in spec.extra_args {
            args.push((*extra).into());
        }

        args.push(spec.image.into());

        podman(args, &format!("criando container {}", spec.name), true)
    }

    fn remove_container(&self, name: &str) -> Result<()> {
        let status = podman(
            ["rm", "-f", name],
            &format!("removendo container {name}"),
            true,
        );

        if status.is_err() {
            warn!("  Não foi possível remover {name} (pode não existir)");
        }

        Ok(())
    }

    fn exec_shell(&self, container: &str, workdir: Option<&Path>) -> Result<()> {
        let mut cmd = Command::new("podman");
        cmd.args(["exec", "-it"]);

        if let Some(dir) = workdir {
            cmd.args(["-w", dir.to_string_lossy().as_ref()]);
        }

        cmd.arg(container)
            .args(["zellij", "attach", "--create", "devobox"]);

        let status = cmd
            .status()
            .with_context(|| format!("abrindo shell em {container}"))?;

        if !status.success() {
            bail!("shell retornou status {:?}", status);
        }

        Ok(())
    }

    fn is_command_available(&self, _cmd: &str) -> bool {
        static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *AVAILABLE.get_or_init(|| {
            Command::new("podman")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
    }

    fn build_image(&self, tag: &str, containerfile: &Path, context_dir: &Path) -> Result<()> {
        podman(
            [
                OsStr::new("build"),
                OsStr::new("--progress=plain"),
                OsStr::new("-t"),
                OsStr::new(tag),
                OsStr::new("-f"),
                containerfile.as_os_str(),
                context_dir.as_os_str(),
            ],
            &format!("construindo imagem {tag} a partir de {:?}", containerfile),
            false, // Mostrar output do build
        )
    }

    fn prune_containers(&self) -> Result<()> {
        podman(
            ["container", "prune", "-f"],
            "removendo containers parados",
            false,
        )
    }

    fn prune_images(&self) -> Result<()> {
        podman(
            ["image", "prune", "-af"],
            "removendo imagens não utilizadas",
            false,
        )
    }

    fn prune_volumes(&self) -> Result<()> {
        podman(["volume", "prune", "-f"], "removendo volumes órfãos", false)
    }

    fn prune_build_cache(&self) -> Result<()> {
        podman(["builder", "prune", "-af"], "limpando cache de build", true)
    }

    fn nuke_system(&self) -> Result<()> {
        info!(" Executando limpeza agressiva (Nuke)...");
        podman(
            ["system", "prune", "-a", "--volumes", "-f"],
            "removendo tudo (imagens, containers, volumes)",
            false,
        )?;
        podman(
            ["builder", "prune", "-a", "-f"],
            "limpando cache de build",
            false,
        )?;
        info!(" Limpeza agressiva concluída!");

        Ok(())
    }

    fn reset_system(&self) -> Result<()> {
        warn!("  ATENÇÃO: System reset irá DELETAR TUDO!");
        warn!("   - Todos containers (rodando ou parados)");
        warn!("   - Todas imagens");
        warn!("   - Todos volumes (incluindo dados persistentes)");
        warn!("   - Reset completo do storage do Podman");
        info!("");
        info!(" Executando system reset...");

        podman(
            ["system", "reset", "-f"],
            "resetando sistema Podman completamente",
            false,
        )?;

        info!(" System reset concluído!");
        info!("   O Podman foi resetado ao estado de fábrica.");

        Ok(())
    }
}

fn get_container_state(name: &str) -> Result<ContainerState> {
    let exists = container_exists(name)?;
    if !exists {
        return Ok(ContainerState::NotCreated);
    }

    let running = container_running(name)?;
    Ok(if running {
        ContainerState::Running
    } else {
        ContainerState::Stopped
    })
}

fn container_running(name: &str) -> Result<bool> {
    let status = Command::new("podman")
        .args([
            "container",
            "inspect",
            name,
            "--format",
            "{{.State.Running}}",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| format!("checando estado do container {name}"))?;

    if !status.status.success() {
        return Ok(false);
    }

    Ok(String::from_utf8_lossy(&status.stdout).trim() == "true")
}

fn container_exists(name: &str) -> Result<bool> {
    let result = podman(
        ["container", "inspect", name],
        &format!("checando existência do container {name}"),
        true,
    );

    Ok(result.is_ok())
}

fn run_podman_cmd<I, S>(args: I, context: &str, quiet: bool) -> Result<(ExitStatus, Option<String>)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = Command::new("podman");
    let args_vec: Vec<std::ffi::OsString> = args
        .into_iter()
        .map(|item| item.as_ref().to_os_string())
        .collect();

    debug!("Executando podman {:?}", args_vec);

    cmd.args(&args_vec);

    if quiet {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().with_context(|| context.to_string())?;

        let stderr_result = if let Some(stderr) = child.stderr.take() {
            use std::io::Read;
            // Limit to 32KB of stderr to prevent OOM on massive failure logs
            let mut buffer = Vec::new();
            let _ = stderr.take(32 * 1024).read_to_end(&mut buffer);
            Some(String::from_utf8_lossy(&buffer).to_string())
        } else {
            None
        };

        let status = child.wait().with_context(|| context.to_string())?;

        let stderr = if !status.success() {
            stderr_result
        } else {
            None
        };
        Ok((status, stderr))
    } else {
        let status = cmd.status().with_context(|| context.to_string())?;
        Ok((status, None))
    }
}

fn podman<I, S>(args: I, context: &str, quiet: bool) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let (status, stderr) = run_podman_cmd(args, context, quiet)?;

    if status.success() {
        Ok(())
    } else {
        let error_msg = stderr.unwrap_or_else(|| "Verifique o output acima".to_string());
        bail!(
            "podman retornou status {:?} ({})\nErro: {}",
            status,
            context,
            error_msg.trim()
        );
    }
}
