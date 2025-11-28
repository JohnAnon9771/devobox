use super::{Container, ContainerSpec};
use anyhow::Result;
use std::path::Path;

/// Trait for container runtime operations
pub trait ContainerRuntime: Send + Sync {
    /// Get the current state of a container
    fn get_container(&self, name: &str) -> Result<Container>;

    /// Start a container
    fn start_container(&self, name: &str) -> Result<()>;

    /// Stop a container
    fn stop_container(&self, name: &str) -> Result<()>;

    /// Create a new container from a spec
    fn create_container(&self, spec: &ContainerSpec) -> Result<()>;

    /// Remove a container
    fn remove_container(&self, name: &str) -> Result<()>;

    /// Execute a shell in a container
    fn exec_shell(&self, container: &str, workdir: Option<&Path>) -> Result<()>;

    /// Check if a command is available
    fn is_command_available(&self, cmd: &str) -> bool;

    /// Build an image
    fn build_image(&self, tag: &str, containerfile: &Path, context_dir: &Path) -> Result<()>;

    /// Prune stopped containers
    fn prune_containers(&self) -> Result<()>;

    /// Prune unused images
    fn prune_images(&self) -> Result<()>;

    /// Prune unused volumes
    fn prune_volumes(&self) -> Result<()>;

    /// Prune build cache
    fn prune_build_cache(&self) -> Result<()>;

    /// Perform an aggressive system cleanup (Nuke)
    fn nuke_system(&self) -> Result<()>;
}
