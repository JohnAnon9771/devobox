use crate::domain::traits::ContainerHealthStatus;
use crate::domain::{ContainerRuntime, ContainerSpec, ContainerState};
use anyhow::{Result, bail};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

pub struct ContainerService {
    runtime: Arc<dyn ContainerRuntime>,
}

impl ContainerService {
    pub fn new(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self { runtime }
    }

    pub fn get_status(&self, name: &str) -> Result<crate::domain::Container> {
        self.runtime.get_container(name)
    }

    pub fn ensure_running(&self, name: &str) -> Result<()> {
        let container = self.runtime.get_container(name)?;

        match container.state {
            ContainerState::Running => Ok(()),
            ContainerState::Stopped => {
                info!(" Iniciando {name}...");
                self.runtime.start_container(name)
            }
            ContainerState::NotCreated => {
                bail!("Container {name} não existe. Rode 'devobox builder build' primeiro.")
            }
        }
    }

    pub fn start(&self, name: &str) -> Result<()> {
        let container = self.runtime.get_container(name)?;

        match container.state {
            ContainerState::Running => {
                warn!("  {name} já está rodando");
                Ok(())
            }
            ContainerState::Stopped => {
                info!(" Iniciando {name}...");
                self.runtime.start_container(name)
            }
            ContainerState::NotCreated => {
                warn!("  Container {name} não existe. Rode 'devobox builder build' primeiro.");
                Ok(())
            }
        }
    }

    pub fn stop(&self, name: &str) -> Result<()> {
        let container = self.runtime.get_container(name)?;

        match container.state {
            ContainerState::Running => {
                info!(" Parando {name}...");
                self.runtime.stop_container(name)
            }
            ContainerState::Stopped | ContainerState::NotCreated => {
                warn!("  {name} já está parado ou não foi criado");
                Ok(())
            }
        }
    }

    pub fn recreate(&self, spec: &ContainerSpec) -> Result<()> {
        self.runtime.remove_container(spec.name)?;
        self.runtime.create_container(spec)
    }

    pub fn exec_shell(&self, container: &str, workdir: Option<&Path>) -> Result<()> {
        self.runtime.exec_shell(container, workdir)
    }

    pub fn is_command_available(&self, cmd: &str) -> bool {
        self.runtime.is_command_available(cmd)
    }

    pub fn get_health_status(&self, name: &str) -> Result<ContainerHealthStatus> {
        self.runtime.get_container_health(name)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{Service, ServiceKind};

    #[test]
    fn test_service_spec_conversion() {
        let svc = Service {
            name: "test_svc".to_string(),
            image: "app:latest".to_string(),
            kind: ServiceKind::Generic,
            ports: vec!["8080:8080".to_string()],
            env: vec!["ENV_VAR=value".to_string()],
            volumes: vec!["/app:/usr/src/app".to_string()],
            healthcheck_command: Some("exit 0".to_string()),
            healthcheck_interval: Some("1s".to_string()),
            healthcheck_timeout: Some("1s".to_string()),
            healthcheck_retries: Some(1),
        };

        let spec = svc.to_spec();
        assert_eq!(spec.name, "test_svc");
    }
}
