use crate::domain::traits::ContainerHealthStatus;
use crate::domain::{Container, ContainerRuntime, ContainerSpec, ContainerState};
use anyhow::{Result, bail};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockContainer {
    pub name: String,
    pub state: ContainerState,
    pub spec: Option<MockContainerSpec>,
    pub health_status: Option<ContainerHealthStatus>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockContainerSpec {
    pub image: String,
    pub ports: Vec<String>,
    pub env: Vec<String>,
    pub healthcheck_command: Option<String>,
    pub healthcheck_interval: Option<String>,
    pub healthcheck_timeout: Option<String>,
    pub healthcheck_retries: Option<u32>,
}

pub struct MockRuntime {
    containers: RwLock<HashMap<String, MockContainer>>,
    commands: RwLock<Vec<String>>,
    fail_on: RwLock<Option<String>>,
}

impl MockRuntime {
    pub fn new() -> Self {
        Self {
            containers: RwLock::new(HashMap::new()),
            commands: RwLock::new(Vec::new()),
            fail_on: RwLock::new(None),
        }
    }

    pub fn add_container(&self, name: &str, state: ContainerState) {
        self.containers.write().unwrap().insert(
            name.to_string(),
            MockContainer {
                name: name.to_string(),
                state,
                spec: None,
                health_status: None,
            },
        );
    }

    #[allow(dead_code)]
    pub fn set_fail_on(&self, operation: &str) {
        *self.fail_on.write().unwrap() = Some(operation.to_string());
    }

    pub fn get_commands(&self) -> Vec<String> {
        self.commands.read().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn container_exists(&self, name: &str) -> bool {
        self.containers.read().unwrap().contains_key(name)
    }

    pub fn get_state(&self, name: &str) -> Option<ContainerState> {
        self.containers
            .read()
            .unwrap()
            .get(name)
            .map(|c| c.state.clone())
    }

    #[allow(dead_code)]
    pub fn set_health_status(&self, name: &str, status: ContainerHealthStatus) {
        if let Some(container) = self.containers.write().unwrap().get_mut(name) {
            container.health_status = Some(status);
        }
    }

    fn record_command(&self, cmd: &str) {
        self.commands.write().unwrap().push(cmd.to_string());
    }

    fn check_fail(&self, operation: &str) -> Result<()> {
        if let Some(ref fail_on) = *self.fail_on.read().unwrap() {
            if fail_on == operation {
                bail!("Mock failure on: {}", operation);
            }
        }
        Ok(())
    }
}

impl Default for MockRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerRuntime for MockRuntime {
    fn get_container(&self, name: &str) -> Result<Container> {
        self.record_command(&format!("get_container:{}", name));
        self.check_fail("get_container")?;

        let state = self
            .containers
            .read()
            .unwrap()
            .get(name)
            .map(|c| c.state.clone())
            .unwrap_or(ContainerState::NotCreated);

        Ok(Container::new(name.to_string(), state))
    }

    fn get_container_health(&self, name: &str) -> Result<ContainerHealthStatus> {
        self.record_command(&format!("get_health:{}", name));
        self.check_fail("get_health")?;

        let containers = self.containers.read().unwrap();
        let status = containers
            .get(name)
            .and_then(|c| c.health_status.clone())
            .unwrap_or(ContainerHealthStatus::NotApplicable); // Default if not explicitly set

        Ok(status)
    }

    fn start_container(&self, name: &str) -> Result<()> {
        self.record_command(&format!("start:{}", name));
        self.check_fail("start")?;

        if let Some(container) = self.containers.write().unwrap().get_mut(name) {
            container.state = ContainerState::Running;
        }
        Ok(())
    }

    fn stop_container(&self, name: &str) -> Result<()> {
        self.record_command(&format!("stop:{}", name));
        self.check_fail("stop")?;

        if let Some(container) = self.containers.write().unwrap().get_mut(name) {
            container.state = ContainerState::Stopped;
        }
        Ok(())
    }

    fn create_container(&self, spec: &ContainerSpec) -> Result<()> {
        self.record_command(&format!("create:{}", spec.name));
        self.check_fail("create")?;

        self.containers.write().unwrap().insert(
            spec.name.to_string(),
            MockContainer {
                name: spec.name.to_string(),
                state: ContainerState::Stopped,
                spec: Some(MockContainerSpec {
                    image: spec.image.to_string(),
                    ports: spec.ports.to_vec(),
                    env: spec.env.to_vec(),
                    healthcheck_command: spec.healthcheck_command.map(|s| s.to_string()),
                    healthcheck_interval: spec.healthcheck_interval.map(|s| s.to_string()),
                    healthcheck_timeout: spec.healthcheck_timeout.map(|s| s.to_string()),
                    healthcheck_retries: spec.healthcheck_retries,
                }),
                health_status: None, // Initial health status is not set
            },
        );
        Ok(())
    }

    fn remove_container(&self, name: &str) -> Result<()> {
        self.record_command(&format!("remove:{}", name));
        self.check_fail("remove")?;

        self.containers.write().unwrap().remove(name);
        Ok(())
    }

    fn exec_shell(&self, container: &str, _workdir: Option<&Path>) -> Result<()> {
        self.record_command(&format!("exec_shell:{}", container));
        self.check_fail("exec_shell")?;
        Ok(())
    }

    fn is_command_available(&self, cmd: &str) -> bool {
        self.record_command(&format!("is_available:{}", cmd));
        true
    }

    fn build_image(&self, tag: &str, _containerfile: &Path, _context_dir: &Path) -> Result<()> {
        self.record_command(&format!("build_image:{}", tag));
        self.check_fail("build_image")?;
        Ok(())
    }

    fn prune_containers(&self) -> Result<()> {
        self.record_command("prune:containers");
        self.check_fail("prune_containers")?;
        Ok(())
    }

    fn prune_images(&self) -> Result<()> {
        self.record_command("prune:images");
        self.check_fail("prune_images")?;
        Ok(())
    }

    fn prune_volumes(&self) -> Result<()> {
        self.record_command("prune:volumes");
        self.check_fail("prune_volumes")?;
        Ok(())
    }

    fn prune_build_cache(&self) -> Result<()> {
        self.record_command("prune:build_cache");
        self.check_fail("prune_build_cache")?;
        Ok(())
    }

    fn nuke_system(&self) -> Result<()> {
        self.record_command("nuke_system");
        self.check_fail("nuke_system")?;
        Ok(())
    }
}
