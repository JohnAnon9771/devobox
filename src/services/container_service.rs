use crate::domain::{ContainerRuntime, ContainerSpec, ContainerState, Database};
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

    pub fn build_image(&self, tag: &str, containerfile: &Path, context_dir: &Path) -> Result<()> {
        self.runtime.build_image(tag, containerfile, context_dir)
    }
}

pub struct DatabaseService {
    container_service: ContainerService,
}

impl DatabaseService {
    pub fn new(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self {
            container_service: ContainerService::new(runtime),
        }
    }

    pub fn start_all(&self, databases: &[Database]) -> Result<()> {
        if databases.is_empty() {
            println!("⚠️  Nenhum banco configurado");
            return Ok(());
        }

        for db in databases {
            self.container_service.start(&db.name)?;
        }
        Ok(())
    }

    pub fn stop_all(&self, databases: &[Database]) -> Result<()> {
        if databases.is_empty() {
            println!("⚠️  Nenhum banco configurado");
            return Ok(());
        }

        for db in databases {
            self.container_service.stop(&db.name)?;
        }
        Ok(())
    }

    pub fn restart_all(&self, databases: &[Database]) -> Result<()> {
        self.stop_all(databases)?;
        self.start_all(databases)
    }

    pub fn start(&self, name: &str, databases: &[Database]) -> Result<()> {
        if !self.is_known_db(name, databases) {
            bail!("Banco '{name}' não está listado em databases.yml");
        }
        self.container_service.start(name)
    }

    pub fn stop(&self, name: &str, databases: &[Database]) -> Result<()> {
        if !self.is_known_db(name, databases) {
            bail!("Banco '{name}' não está listado em databases.yml");
        }
        self.container_service.stop(name)
    }

    pub fn restart(&self, name: &str, databases: &[Database]) -> Result<()> {
        self.stop(name, databases)?;
        self.start(name, databases)
    }

    fn is_known_db(&self, name: &str, databases: &[Database]) -> bool {
        databases.iter().any(|db| db.name == name)
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
    fn test_database_service_start_all() {
        let mock = Arc::new(MockRuntime::new());
        mock.add_container("db1", ContainerState::Stopped);
        mock.add_container("db2", ContainerState::Stopped);

        let service = DatabaseService::new(mock.clone());
        let databases = vec![
            Database {
                name: "db1".to_string(),
                image: "postgres:15".to_string(),
                ports: vec![],
                env: vec![],
                volumes: vec![],
            },
            Database {
                name: "db2".to_string(),
                image: "redis:7".to_string(),
                ports: vec![],
                env: vec![],
                volumes: vec![],
            },
        ];

        let result = service.start_all(&databases);
        assert!(result.is_ok());

        assert_eq!(mock.get_state("db1"), Some(ContainerState::Running));
        assert_eq!(mock.get_state("db2"), Some(ContainerState::Running));
    }

    #[test]
    fn test_database_service_validates_known_database() {
        let mock = Arc::new(MockRuntime::new());
        let service = DatabaseService::new(mock);

        let databases = vec![Database {
            name: "db1".to_string(),
            image: "postgres:15".to_string(),
            ports: vec![],
            env: vec![],
            volumes: vec![],
        }];

        let result = service.start("unknown", &databases);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("não está listado"));
    }

    #[test]
    fn test_database_service_restart() {
        let mock = Arc::new(MockRuntime::new());
        mock.add_container("db1", ContainerState::Running);

        let service = DatabaseService::new(mock.clone());
        let databases = vec![Database {
            name: "db1".to_string(),
            image: "postgres:15".to_string(),
            ports: vec![],
            env: vec![],
            volumes: vec![],
        }];

        let result = service.restart("db1", &databases);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"stop:db1".to_string()));
        assert!(commands.contains(&"start:db1".to_string()));
    }

    #[test]
    fn test_stop_already_stopped_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Stopped);

        let result = service.stop("test");
        assert!(result.is_ok());

        // Should not call stop on already stopped container
        let commands = mock.get_commands();
        assert!(!commands.contains(&"stop:test".to_string()));
    }

    #[test]
    fn test_start_already_running_container() {
        let (service, mock) = create_test_service();
        mock.add_container("test", ContainerState::Running);

        let result = service.start("test");
        assert!(result.is_ok());

        // Should not call start on already running container
        let commands = mock.get_commands();
        assert!(!commands.contains(&"start:test".to_string()));
    }

    #[test]
    fn test_start_nonexistent_container() {
        let (service, _mock) = create_test_service();

        let result = service.start("missing");
        assert!(result.is_ok()); // Should not fail, just print warning
    }

    #[test]
    fn test_database_service_stop_all() {
        let mock = Arc::new(MockRuntime::new());
        mock.add_container("db1", ContainerState::Running);
        mock.add_container("db2", ContainerState::Running);

        let service = DatabaseService::new(mock.clone());
        let databases = vec![
            Database {
                name: "db1".to_string(),
                image: "postgres:15".to_string(),
                ports: vec![],
                env: vec![],
                volumes: vec![],
            },
            Database {
                name: "db2".to_string(),
                image: "redis:7".to_string(),
                ports: vec![],
                env: vec![],
                volumes: vec![],
            },
        ];

        let result = service.stop_all(&databases);
        assert!(result.is_ok());

        assert_eq!(mock.get_state("db1"), Some(ContainerState::Stopped));
        assert_eq!(mock.get_state("db2"), Some(ContainerState::Stopped));
    }

    #[test]
    fn test_database_service_with_empty_list() {
        let mock = Arc::new(MockRuntime::new());
        let service = DatabaseService::new(mock);
        let databases = vec![];

        let result = service.start_all(&databases);
        assert!(result.is_ok());

        let result = service.stop_all(&databases);
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

        // Start -> Stop -> Start sequence
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
