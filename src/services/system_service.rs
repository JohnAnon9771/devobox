use crate::domain::ContainerRuntime;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

/// System-wide Podman operations (build, prune)
pub struct SystemService {
    runtime: Arc<dyn ContainerRuntime>,
}

impl SystemService {
    pub fn new(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self { runtime }
    }

    pub fn build_image(&self, tag: &str, containerfile: &Path, context: &Path) -> Result<()> {
        self.runtime.build_image(tag, containerfile, context)
    }

    pub fn prune_containers(&self) -> Result<()> {
        self.runtime.prune_containers()
    }

    pub fn prune_images(&self) -> Result<()> {
        self.runtime.prune_images()
    }

    pub fn prune_volumes(&self) -> Result<()> {
        self.runtime.prune_volumes()
    }

    pub fn prune_build_cache(&self) -> Result<()> {
        self.runtime.prune_build_cache()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockRuntime;

    #[test]
    fn test_prune_containers() {
        let mock = Arc::new(MockRuntime::new());
        let service = SystemService::new(mock.clone());

        let result = service.prune_containers();
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:containers".to_string()));
    }

    #[test]
    fn test_prune_images() {
        let mock = Arc::new(MockRuntime::new());
        let service = SystemService::new(mock.clone());

        let result = service.prune_images();
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:images".to_string()));
    }

    #[test]
    fn test_prune_volumes() {
        let mock = Arc::new(MockRuntime::new());
        let service = SystemService::new(mock.clone());

        let result = service.prune_volumes();
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:volumes".to_string()));
    }

    #[test]
    fn test_prune_build_cache() {
        let mock = Arc::new(MockRuntime::new());
        let service = SystemService::new(mock.clone());

        let result = service.prune_build_cache();
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:build_cache".to_string()));
    }

    #[test]
    fn test_build_image() {
        let mock = Arc::new(MockRuntime::new());
        let service = SystemService::new(mock.clone());

        let containerfile = std::path::Path::new("/tmp/Containerfile");
        let context = std::path::Path::new("/tmp");

        let result = service.build_image("test-img", containerfile, context);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"build_image:test-img".to_string()));
    }
}
