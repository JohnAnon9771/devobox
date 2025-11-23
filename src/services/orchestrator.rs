use crate::services::{ContainerService, SystemService};
use anyhow::Result;
use std::sync::Arc;

/// Orquestrador central para workflows complexos
///
/// Combina ContainerService e SystemService para implementar
/// workflows de alto nível que envolvem múltiplos containers
/// e operações de sistema.
pub struct Orchestrator {
    container_service: Arc<ContainerService>,
    system_service: Arc<SystemService>,
}

#[derive(Debug, Clone)]
pub struct CleanupOptions {
    pub containers: bool,
    pub images: bool,
    pub volumes: bool,
    pub build_cache: bool,
}

impl CleanupOptions {
    pub fn all() -> Self {
        Self {
            containers: true,
            images: true,
            volumes: true,
            build_cache: true,
        }
    }

    pub fn none() -> Self {
        Self {
            containers: false,
            images: false,
            volumes: false,
            build_cache: false,
        }
    }
}

impl Orchestrator {
    pub fn new(
        container_service: Arc<ContainerService>,
        system_service: Arc<SystemService>,
    ) -> Self {
        Self {
            container_service,
            system_service,
        }
    }

    /// Para todos os containers da lista, ignorando erros individuais.
    /// Continua tentando parar todos mesmo se algum falhar.
    ///
    /// # Arguments
    /// * `container_names` - Lista de nomes dos containers a serem parados
    ///
    /// # Returns
    /// Ok se a operação foi tentada para todos (mesmo com falhas individuais)
    pub fn stop_all(&self, container_names: &[String]) -> Result<()> {
        if container_names.is_empty() {
            return Ok(());
        }

        println!("🧹 Encerrando todos os containers...");

        for name in container_names {
            print!("  💤 Parando {name}...");
            match self.container_service.stop(name) {
                Ok(_) => println!(" ✓"),
                Err(e) => println!(" ⚠️  Falha: {}", e),
            }
        }

        println!("✅ Containers encerrados");
        Ok(())
    }

    /// Inicia todos os containers da lista, ignorando erros individuais.
    /// Continua tentando iniciar todos mesmo se algum falhar.
    ///
    /// # Arguments
    /// * `container_names` - Lista de nomes dos containers a serem iniciados
    ///
    /// # Returns
    /// Ok se a operação foi tentada para todos (mesmo com falhas individuais)
    pub fn start_all(&self, container_names: &[String]) -> Result<()> {
        if container_names.is_empty() {
            return Ok(());
        }

        println!("🚀 Iniciando todos os containers...");

        for name in container_names {
            print!("  🔌 Iniciando {name}...");
            match self.container_service.start(name) {
                Ok(_) => println!(" ✓"),
                Err(e) => println!(" ⚠️  Falha: {}", e),
            }
        }

        println!("✅ Containers iniciados");
        Ok(())
    }

    /// Executa limpeza de recursos do Podman baseado nas opções fornecidas.
    /// Continua tentando limpar todos os recursos selecionados mesmo se algum falhar.
    ///
    /// # Arguments
    /// * `options` - Opções de limpeza (quais recursos limpar)
    ///
    /// # Returns
    /// Ok se a operação foi tentada (mesmo com falhas individuais)
    pub fn cleanup(&self, options: &CleanupOptions) -> Result<()> {
        println!("🧹 Limpando recursos do Podman...");

        if options.containers {
            print!("  ⏳ Removendo containers parados...");
            match self.system_service.prune_containers() {
                Ok(_) => println!(" ✓"),
                Err(e) => println!(" ⚠️  Falha: {}", e),
            }
        }

        if options.images {
            print!("  ⏳ Removendo imagens não utilizadas...");
            match self.system_service.prune_images() {
                Ok(_) => println!(" ✓"),
                Err(e) => println!(" ⚠️  Falha: {}", e),
            }
        }

        if options.volumes {
            print!("  ⏳ Removendo volumes órfãos...");
            match self.system_service.prune_volumes() {
                Ok(_) => println!(" ✓"),
                Err(e) => println!(" ⚠️  Falha: {}", e),
            }
        }

        if options.build_cache {
            print!("  ⏳ Limpando cache de build...");
            match self.system_service.prune_build_cache() {
                Ok(_) => println!(" ✓"),
                Err(e) => println!(" ⚠️  Falha: {}", e),
            }
        }

        println!("✨ Limpeza concluída!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ContainerState;
    use crate::test_support::MockRuntime;

    fn create_test_orchestrator() -> (Orchestrator, Arc<MockRuntime>) {
        let mock = Arc::new(MockRuntime::new());
        let container_service = Arc::new(ContainerService::new(mock.clone()));
        let system_service = Arc::new(SystemService::new(mock.clone()));
        let orchestrator = Orchestrator::new(container_service, system_service);
        (orchestrator, mock)
    }

    #[test]
    fn test_stop_all_stops_all_containers() {
        let (orchestrator, mock) = create_test_orchestrator();

        mock.add_container("devobox", ContainerState::Running);
        mock.add_container("pg", ContainerState::Running);
        mock.add_container("redis", ContainerState::Running);

        let containers = vec!["devobox".to_string(), "pg".to_string(), "redis".to_string()];

        let result = orchestrator.stop_all(&containers);
        assert!(result.is_ok());

        assert_eq!(mock.get_state("devobox"), Some(ContainerState::Stopped));
        assert_eq!(mock.get_state("pg"), Some(ContainerState::Stopped));
        assert_eq!(mock.get_state("redis"), Some(ContainerState::Stopped));

        let commands = mock.get_commands();
        assert!(commands.contains(&"stop:devobox".to_string()));
        assert!(commands.contains(&"stop:pg".to_string()));
        assert!(commands.contains(&"stop:redis".to_string()));
    }

    #[test]
    fn test_stop_all_handles_already_stopped() {
        let (orchestrator, mock) = create_test_orchestrator();

        mock.add_container("devobox", ContainerState::Running);
        mock.add_container("pg", ContainerState::Stopped);

        let containers = vec!["devobox".to_string(), "pg".to_string()];

        let result = orchestrator.stop_all(&containers);
        assert!(result.is_ok());

        assert_eq!(mock.get_state("devobox"), Some(ContainerState::Stopped));
        assert_eq!(mock.get_state("pg"), Some(ContainerState::Stopped));
    }

    #[test]
    fn test_stop_all_continues_on_failure() {
        let (orchestrator, mock) = create_test_orchestrator();

        mock.add_container("devobox", ContainerState::Running);
        mock.add_container("pg", ContainerState::Running);
        mock.set_fail_on("stop");

        let containers = vec!["devobox".to_string(), "pg".to_string()];

        let result = orchestrator.stop_all(&containers);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"stop:devobox".to_string()));
        assert!(commands.contains(&"stop:pg".to_string()));
    }

    #[test]
    fn test_stop_all_with_empty_list() {
        let (orchestrator, _mock) = create_test_orchestrator();

        let containers: Vec<String> = vec![];

        let result = orchestrator.stop_all(&containers);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_all_starts_all_containers() {
        let (orchestrator, mock) = create_test_orchestrator();

        mock.add_container("devobox", ContainerState::Stopped);
        mock.add_container("pg", ContainerState::Stopped);
        mock.add_container("redis", ContainerState::Stopped);

        let containers = vec!["devobox".to_string(), "pg".to_string(), "redis".to_string()];

        let result = orchestrator.start_all(&containers);
        assert!(result.is_ok());

        assert_eq!(mock.get_state("devobox"), Some(ContainerState::Running));
        assert_eq!(mock.get_state("pg"), Some(ContainerState::Running));
        assert_eq!(mock.get_state("redis"), Some(ContainerState::Running));

        let commands = mock.get_commands();
        assert!(commands.contains(&"start:devobox".to_string()));
        assert!(commands.contains(&"start:pg".to_string()));
        assert!(commands.contains(&"start:redis".to_string()));
    }

    #[test]
    fn test_start_all_handles_already_running() {
        let (orchestrator, mock) = create_test_orchestrator();

        mock.add_container("devobox", ContainerState::Stopped);
        mock.add_container("pg", ContainerState::Running);

        let containers = vec!["devobox".to_string(), "pg".to_string()];

        let result = orchestrator.start_all(&containers);
        assert!(result.is_ok());

        assert_eq!(mock.get_state("devobox"), Some(ContainerState::Running));
        assert_eq!(mock.get_state("pg"), Some(ContainerState::Running));
    }

    #[test]
    fn test_start_all_continues_on_failure() {
        let (orchestrator, mock) = create_test_orchestrator();

        mock.add_container("devobox", ContainerState::Stopped);
        mock.add_container("pg", ContainerState::Stopped);
        mock.set_fail_on("start");

        let containers = vec!["devobox".to_string(), "pg".to_string()];

        let result = orchestrator.start_all(&containers);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"start:devobox".to_string()));
        assert!(commands.contains(&"start:pg".to_string()));
    }

    #[test]
    fn test_start_all_with_empty_list() {
        let (orchestrator, _mock) = create_test_orchestrator();

        let containers: Vec<String> = vec![];

        let result = orchestrator.start_all(&containers);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_all() {
        let (orchestrator, mock) = create_test_orchestrator();
        let options = CleanupOptions::all();

        let result = orchestrator.cleanup(&options);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:containers".to_string()));
        assert!(commands.contains(&"prune:images".to_string()));
        assert!(commands.contains(&"prune:volumes".to_string()));
        assert!(commands.contains(&"prune:build_cache".to_string()));
    }

    #[test]
    fn test_cleanup_selective_containers_only() {
        let (orchestrator, mock) = create_test_orchestrator();
        let options = CleanupOptions {
            containers: true,
            images: false,
            volumes: false,
            build_cache: false,
        };

        let result = orchestrator.cleanup(&options);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:containers".to_string()));
        assert!(!commands.contains(&"prune:images".to_string()));
        assert!(!commands.contains(&"prune:volumes".to_string()));
        assert!(!commands.contains(&"prune:build_cache".to_string()));
    }

    #[test]
    fn test_cleanup_selective_images_and_cache() {
        let (orchestrator, mock) = create_test_orchestrator();
        let options = CleanupOptions {
            containers: false,
            images: true,
            volumes: false,
            build_cache: true,
        };

        let result = orchestrator.cleanup(&options);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(!commands.contains(&"prune:containers".to_string()));
        assert!(commands.contains(&"prune:images".to_string()));
        assert!(!commands.contains(&"prune:volumes".to_string()));
        assert!(commands.contains(&"prune:build_cache".to_string()));
    }

    #[test]
    fn test_cleanup_none() {
        let (orchestrator, mock) = create_test_orchestrator();
        let options = CleanupOptions::none();

        let result = orchestrator.cleanup(&options);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(!commands.contains(&"prune:containers".to_string()));
        assert!(!commands.contains(&"prune:images".to_string()));
        assert!(!commands.contains(&"prune:volumes".to_string()));
        assert!(!commands.contains(&"prune:build_cache".to_string()));
    }

    #[test]
    fn test_cleanup_continues_on_individual_failures() {
        let (orchestrator, mock) = create_test_orchestrator();

        // Configura falha no prune de imagens
        mock.set_fail_on("prune_images");

        let options = CleanupOptions::all();

        // Cleanup não deve falhar completamente
        let result = orchestrator.cleanup(&options);
        assert!(result.is_ok());

        // Outros comandos de prune devem ter sido executados
        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:containers".to_string()));
        assert!(commands.contains(&"prune:volumes".to_string()));
        assert!(commands.contains(&"prune:build_cache".to_string()));
    }
}
