use crate::domain::{ContainerRuntime, ContainerSpec, ContainerState};
use anyhow::{Result, bail};
use std::path::Path;
use std::sync::Arc;

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
                println!("🔌 Iniciando {name}...");
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
                println!("⚠️  {name} já está rodando");
                Ok(())
            }
            ContainerState::Stopped => {
                println!("🔌 Iniciando {name}...");
                self.runtime.start_container(name)
            }
            ContainerState::NotCreated => {
                println!("⚠️  Container {name} não existe. Rode 'devobox builder build' primeiro.");
                Ok(())
            }
        }
    }

    pub fn stop(&self, name: &str) -> Result<()> {
        let container = self.runtime.get_container(name)?;

        match container.state {
            ContainerState::Running => {
                println!("💤 Parando {name}...");
                self.runtime.stop_container(name)
            }
            ContainerState::Stopped | ContainerState::NotCreated => {
                println!("⚠️  {name} já está parado ou não foi criado");
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Database;
    use crate::test_support::MockRuntime;

    fn create_test_service() -> (ContainerService, Arc<MockRuntime>) {
        let mock = Arc::new(MockRuntime::new());
        let service = ContainerService::new(mock.clone());
        (service, mock)
    }

    #[test]
    fn test_ensure_running_starts_stopped_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Stopped);

        let result = service.ensure_running("test");
        assert!(result.is_ok());
        assert_eq!(mock.get_state("test"), Some(ContainerState::Running));
    }

    #[test]
    fn test_ensure_running_fails_if_not_created() {
        let (service, _mock) = create_test_service();

        let result = service.ensure_running("missing");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("não existe"));
    }

    #[test]
    fn test_ensure_running_does_nothing_if_already_running() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Running);

        let result = service.ensure_running("test");
        assert!(result.is_ok());
        assert_eq!(mock.get_state("test"), Some(ContainerState::Running));
    }

    #[test]
    fn test_start_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Stopped);

        let result = service.start("test");
        assert!(result.is_ok());
        assert_eq!(mock.get_state("test"), Some(ContainerState::Running));

        let commands = mock.get_commands();
        assert!(commands.contains(&"start:test".to_string()));
    }

    #[test]
    fn test_stop_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Running);

        let result = service.stop("test");
        assert!(result.is_ok());
        assert_eq!(mock.get_state("test"), Some(ContainerState::Stopped));

        let commands = mock.get_commands();
        assert!(commands.contains(&"stop:test".to_string()));
    }

    #[test]
    fn test_recreate_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Running);

        let spec = ContainerSpec {
            name: "test",
            image: "test:latest",
            ports: &[],
            env: &[],
            network: None,
            userns: None,
            security_opt: None,
            workdir: None,
            volumes: &[],
            extra_args: &[],
        };

        let result = service.recreate(&spec);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"remove:test".to_string()));
        assert!(commands.contains(&"create:test".to_string()));
    }

    #[test]
    fn test_stop_already_stopped_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Stopped);

        let result = service.stop("test");
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(!commands.contains(&"stop:test".to_string()));
    }

    #[test]
    fn test_start_already_running_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Running);

        let result = service.start("test");
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(!commands.contains(&"start:test".to_string()));
    }

    #[test]
    fn test_start_nonexistent_container() {
        let (service, _mock) = create_test_service();

        let result = service.start("missing");
        assert!(result.is_ok());
    }

    #[test]
    fn test_container_spec_conversion() {
        let db = Database {
            name: "test_db".to_string(),
            image: "postgres:15".to_string(),
            ports: vec!["5432:5432".to_string()],
            env: vec!["POSTGRES_PASSWORD=secret".to_string()],
            volumes: vec!["/data:/var/lib/postgresql".to_string()],
        };

        let spec = db.to_spec();
        assert_eq!(spec.name, "test_db");
        assert_eq!(spec.image, "postgres:15");
        assert_eq!(spec.ports.len(), 1);
        assert_eq!(spec.env.len(), 1);
        assert_eq!(spec.volumes.len(), 1);
    }

    #[test]
    fn test_multiple_operations_sequence() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Stopped);

        assert!(service.start("test").is_ok());
        assert_eq!(mock.get_state("test"), Some(ContainerState::Running));

        assert!(service.stop("test").is_ok());
        assert_eq!(mock.get_state("test"), Some(ContainerState::Stopped));

        assert!(service.start("test").is_ok());
        assert_eq!(mock.get_state("test"), Some(ContainerState::Running));

        let commands = mock.get_commands();
        assert_eq!(
            commands.iter().filter(|c| c.starts_with("start:")).count(),
            2
        );
        assert_eq!(
            commands.iter().filter(|c| c.starts_with("stop:")).count(),
            1
        );
    }
}
