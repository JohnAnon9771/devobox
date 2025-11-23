use crate::domain::{Container, ContainerRuntime, ContainerSpec, ContainerState};
use anyhow::{Context, Result, bail};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

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

    fn start_container(&self, name: &str) -> Result<()> {
        podman(["start", name], &format!("iniciando container {name}"))
    }

    fn stop_container(&self, name: &str) -> Result<()> {
        podman(["stop", name], &format!("parando container {name}"))
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

        for extra in spec.extra_args {
            args.push((*extra).into());
        }

        args.push(spec.image.into());

        podman(args, &format!("criando container {}", spec.name))
    }

    fn remove_container(&self, name: &str) -> Result<()> {
        let status = podman_status(["rm", "-f", name], &format!("removendo container {name}"))?;

        if !status.success() {
            println!("⚠️  Não foi possível remover {name} (pode não existir)");
        }

        Ok(())
    }

    fn exec_shell(&self, container: &str, workdir: Option<&Path>) -> Result<()> {
        let mut cmd = Command::new("podman");
        cmd.args(["exec", "-it"]);

        if let Some(dir) = workdir {
            cmd.args(["-w", dir.to_string_lossy().as_ref()]);
        }

        cmd.arg(container).arg("bash");

        let status = cmd
            .status()
            .with_context(|| format!("abrindo shell em {container}"))?;

        if !status.success() {
            bail!("shell retornou status {:?}", status);
        }

        Ok(())
    }

    fn is_command_available(&self, _cmd: &str) -> bool {
        Command::new("podman")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn build_image(&self, tag: &str, containerfile: &Path, context_dir: &Path) -> Result<()> {
        podman(
            [
                OsStr::new("build"),
                OsStr::new("-t"),
                OsStr::new(tag),
                OsStr::new("-f"),
                containerfile.as_os_str(),
                context_dir.as_os_str(),
            ],
            &format!("construindo imagem {tag} a partir de {:?}", containerfile),
        )
    }

    fn prune_containers(&self) -> Result<()> {
        podman(["container", "prune", "-f"], "removendo containers parados")
    }

    fn prune_images(&self) -> Result<()> {
        podman(
            ["image", "prune", "-af"],
            "removendo imagens não utilizadas",
        )
    }

    fn prune_volumes(&self) -> Result<()> {
        podman(["volume", "prune", "-f"], "removendo volumes órfãos")
    }

    fn prune_build_cache(&self) -> Result<()> {
        let status = podman_status(["builder", "prune", "-af"], "limpando cache de build");
        match status {
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        }
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
        .output()
        .with_context(|| format!("checando estado do container {name}"))?;

    if !status.status.success() {
        return Ok(false);
    }

    Ok(String::from_utf8_lossy(&status.stdout).trim() == "true")
}

fn container_exists(name: &str) -> Result<bool> {
    Ok(podman_status(
        ["container", "inspect", name],
        &format!("checando existência do container {name}"),
    )?
    .success())
}

fn podman<I, S>(args: I, context: &str) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = podman_status(args, context)?;
    ensure_success(status, context)
}

fn podman_status<I, S>(args: I, context: &str) -> Result<ExitStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("podman")
        .args(args.into_iter().map(|item| item.as_ref().to_os_string()))
        .status()
        .with_context(|| context.to_string())
}

fn ensure_success(status: ExitStatus, context: &str) -> Result<()> {
    if status.success() {
        return Ok(());
    }

    bail!("podman retornou status {:?} ({context})", status)
}
