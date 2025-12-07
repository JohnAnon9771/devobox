use crate::domain::Service;
use crate::domain::traits::ContainerHealthStatus;
use crate::services::{ContainerService, SystemService};
use anyhow::Result;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Orchestrates complex workflows involving multiple containers and system operations
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

    /// Stops all containers in the list, continuing even if individual operations fail
    pub fn stop_all(&self, container_names: &[String]) -> Result<()> {
        if container_names.is_empty() {
            return Ok(());
        }

        info!(" Encerrando todos os containers...");

        for name in container_names {
            match self.container_service.stop(name) {
                Ok(_) => debug!("Container {} parado com sucesso", name),

                Err(e) => error!("  Falha ao parar {}: {}", name, e),
            }
        }

        info!(" Containers encerrados");

        Ok(())
    }

    /// Starts all containers in the list, continuing even if individual operations fail
    pub fn start_all(&self, services: &[Service]) -> Result<()> {
        if services.is_empty() {
            return Ok(());
        }

        info!(" Iniciando todos os serviços...");

        for svc in services {
            match self.container_service.start(&svc.name) {
                Ok(_) => debug!("Serviço {} iniciado", svc.name),

                Err(e) => error!("  Falha ao iniciar {}: {}", svc.name, e),
            }
        }

        info!(" Verificando healthchecks...");

        for svc in services {
            if svc.healthcheck_command.is_some() {
                info!("ﱮ Aguardando {} ficar saudável...", svc.name);

                let mut retries = svc.healthcheck_retries.unwrap_or(3);

                let interval_str = svc.healthcheck_interval.as_deref().unwrap_or("1s");

                let interval = parse_duration(interval_str).unwrap_or(Duration::from_secs(1));

                loop {
                    match self.container_service.get_health_status(&svc.name) {
                        Ok(ContainerHealthStatus::Healthy) => {
                            info!(" {} está saudável!", svc.name);

                            break;
                        }

                        Ok(ContainerHealthStatus::Starting) => {
                            debug!("{} ainda iniciando...", svc.name);
                        }

                        Ok(ContainerHealthStatus::Unhealthy) => {
                            warn!(" {} reportou unhealthy.", svc.name);

                            if retries == 0 {
                                anyhow::bail!(
                                    "Serviço '{}' falhou no healthcheck após várias tentativas.",
                                    svc.name
                                );
                            }

                            retries -= 1;
                        }

                        Ok(ContainerHealthStatus::NotApplicable) => {
                            warn!(
                                " {} não tem healthcheck aplicável. Prosseguindo.",
                                svc.name
                            );

                            break;
                        }

                        Err(e) => {
                            error!(" Erro ao verificar healthcheck de {}: {}", svc.name, e);

                            if retries == 0 {
                                anyhow::bail!(
                                    "Erro persistente ao verificar healthcheck do serviço '{}'.",
                                    svc.name
                                );
                            }

                            retries -= 1;
                        }

                        _ => {
                            debug!("Status desconhecido para {}", svc.name);
                        }
                    }

                    thread::sleep(interval);
                }
            } else {
                info!(
                    " Serviço '{}' sem healthcheck configurado. Prosseguindo.",
                    svc.name
                );
            }
        }

        info!(" Todos os serviços iniciados e saudáveis (ou sem healthcheck).");

        Ok(())
    }

    /// Cleans up Podman resources based on options, continuing even if individual operations fail
    pub fn cleanup(&self, options: &CleanupOptions) -> Result<()> {
        info!(" Limpando recursos do Podman...");

        if options.containers {
            info!(" Removendo containers parados...");

            match self.system_service.prune_containers() {
                Ok(_) => debug!("Containers removidos"),

                Err(e) => warn!("Falha ao remover containers: {}", e),
            }
        }

        if options.images {
            info!(" Removendo imagens não utilizadas...");

            match self.system_service.prune_images() {
                Ok(_) => debug!("Imagens removidas"),

                Err(e) => warn!("Falha ao remover imagens: {}", e),
            }
        }

        if options.volumes {
            info!(" Removendo volumes órfãos...");

            match self.system_service.prune_volumes() {
                Ok(_) => debug!("Volumes removidos"),

                Err(e) => warn!("Falha ao remover volumes: {}", e),
            }
        }

        if options.build_cache {
            info!(" Limpando cache de build...");

            match self.system_service.prune_build_cache() {
                Ok(_) => debug!("Cache limpo"),

                Err(e) => warn!("Falha ao limpar cache: {}", e),
            }
        }

        info!(" Limpeza concluída!");

        Ok(())
    }

    /// Performs a "Nuke" cleanup (aggressive system reset)
    pub fn nuke_system(&self) -> Result<()> {
        self.system_service.nuke_system()
    }
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix('s') {
        let secs: u64 = stripped.parse()?;
        Ok(Duration::from_secs(secs))
    } else if let Some(stripped) = s.strip_suffix('m') {
        let mins: u64 = stripped.parse()?;
        Ok(Duration::from_secs(mins * 60))
    } else {
        Err(anyhow::anyhow!("Formato de duração inválido: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ContainerState, ServiceKind};
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

        let svc1 = Service {
            name: "pg".to_string(),
            image: "postgres".to_string(),
            kind: ServiceKind::Database,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
        };
        let svc2 = Service {
            name: "redis".to_string(),
            image: "redis".to_string(),
            kind: ServiceKind::Database,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
        };

        mock.add_container(&svc1.name, ContainerState::Stopped);
        mock.add_container(&svc2.name, ContainerState::Stopped);

        let services = vec![svc1.clone(), svc2.clone()];

        let result = orchestrator.start_all(&services);
        assert!(result.is_ok());

        assert_eq!(mock.get_state(&svc1.name), Some(ContainerState::Running));
        assert_eq!(mock.get_state(&svc2.name), Some(ContainerState::Running));

        let commands = mock.get_commands();
        assert!(commands.contains(&format!("start:{}", svc1.name)));
        assert!(commands.contains(&format!("start:{}", svc2.name)));
    }

    #[test]
    fn test_start_all_handles_already_running() {
        let (orchestrator, mock) = create_test_orchestrator();

        let svc1 = Service {
            name: "pg".to_string(),
            image: "postgres".to_string(),
            kind: ServiceKind::Database,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
        };
        let svc2 = Service {
            name: "devobox".to_string(),
            image: "devobox-img".to_string(),
            kind: ServiceKind::Generic,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
        };

        mock.add_container(&svc1.name, ContainerState::Running);
        mock.add_container(&svc2.name, ContainerState::Stopped);

        let services = vec![svc1.clone(), svc2.clone()];

        let result = orchestrator.start_all(&services);
        assert!(result.is_ok());

        assert_eq!(mock.get_state(&svc1.name), Some(ContainerState::Running));
        assert_eq!(mock.get_state(&svc2.name), Some(ContainerState::Running));
    }

    #[test]
    fn test_start_all_continues_on_failure() {
        let (orchestrator, mock) = create_test_orchestrator();

        let svc1 = Service {
            name: "pg".to_string(),
            image: "postgres".to_string(),
            kind: ServiceKind::Database,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
        };
        let svc2 = Service {
            name: "redis".to_string(),
            image: "redis".to_string(),
            kind: ServiceKind::Database,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: None,
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
        };

        mock.add_container(&svc1.name, ContainerState::Stopped);
        mock.add_container(&svc2.name, ContainerState::Stopped);
        mock.set_fail_on("start");

        let services = vec![svc1.clone(), svc2.clone()];

        let result = orchestrator.start_all(&services);
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&format!("start:{}", svc1.name)));
        assert!(commands.contains(&format!("start:{}", svc2.name)));
    }

    #[test]
    fn test_start_all_with_empty_list() {
        let (orchestrator, _mock) = create_test_orchestrator();

        let services: Vec<Service> = vec![];

        let result = orchestrator.start_all(&services);
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

        mock.set_fail_on("prune_images");

        let options = CleanupOptions::all();

        let result = orchestrator.cleanup(&options);
        assert!(result.is_ok());
        let commands = mock.get_commands();
        assert!(commands.contains(&"prune:containers".to_string()));
        assert!(commands.contains(&"prune:volumes".to_string()));
        assert!(commands.contains(&"prune:build_cache".to_string()));
    }

    #[test]
    fn test_nuke_system() {
        let (orchestrator, mock) = create_test_orchestrator();

        let result = orchestrator.nuke_system();
        assert!(result.is_ok());

        let commands = mock.get_commands();
        assert!(commands.contains(&"nuke_system".to_string()));
    }

    #[test]
    fn test_start_all_waits_for_healthy_service() {
        let (orchestrator, mock) = create_test_orchestrator();

        let svc = Service {
            name: "web_app".to_string(),
            image: "app:latest".to_string(),
            kind: ServiceKind::Generic,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: Some("echo ok".to_string()),
            healthcheck_interval: Some("1s".to_string()),
            healthcheck_timeout: Some("1s".to_string()),
            healthcheck_retries: Some(1),
        };

        mock.add_container(&svc.name, ContainerState::Stopped);
        mock.set_health_status(&svc.name, ContainerHealthStatus::Starting); // Initially starting

        let services = vec![svc.clone()];

        // Simulate health status change
        let mock_clone = mock.clone();
        let svc_name_clone = svc.name.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            mock_clone.set_health_status(&svc_name_clone, ContainerHealthStatus::Healthy);
        });

        let result = orchestrator.start_all(&services);
        assert!(result.is_ok());

        assert_eq!(mock.get_state(&svc.name), Some(ContainerState::Running));
        let commands = mock.get_commands();
        assert!(commands.contains(&format!("start:{}", svc.name)));
        // Should contain get_health calls
        assert!(commands.iter().any(|c| c.starts_with("get_health:")));
    }

    #[test]
    fn test_start_all_fails_on_unhealthy_service_after_retries() {
        let (orchestrator, mock) = create_test_orchestrator();

        let svc = Service {
            name: "db_svc".to_string(),
            image: "db:latest".to_string(),
            kind: ServiceKind::Database,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: Some("pg_isready".to_string()),
            healthcheck_interval: Some("1s".to_string()),
            healthcheck_timeout: Some("1s".to_string()),
            healthcheck_retries: Some(1), // Fails after 1 retry
        };

        mock.add_container(&svc.name, ContainerState::Stopped);
        mock.set_health_status(&svc.name, ContainerHealthStatus::Unhealthy); // Always unhealthy

        let services = vec![svc.clone()];

        let result = orchestrator.start_all(&services);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("falhou no healthcheck")
        );

        let commands = mock.get_commands();
        assert!(commands.contains(&format!("start:{}", svc.name)));
        assert!(commands.iter().any(|c| c.starts_with("get_health:")));
    }

    #[test]
    fn test_start_all_continues_for_service_without_healthcheck() {
        let (orchestrator, mock) = create_test_orchestrator();

        let svc = Service {
            name: "no_hc_app".to_string(),
            image: "simple:latest".to_string(),
            kind: ServiceKind::Generic,
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            healthcheck_command: None, // No healthcheck
            healthcheck_interval: None,
            healthcheck_timeout: None,
            healthcheck_retries: None,
        };

        mock.add_container(&svc.name, ContainerState::Stopped);

        let services = vec![svc.clone()];

        let result = orchestrator.start_all(&services);
        assert!(result.is_ok());

        assert_eq!(mock.get_state(&svc.name), Some(ContainerState::Running));
        let commands = mock.get_commands();
        assert!(commands.contains(&format!("start:{}", svc.name)));
        assert!(
            !commands
                .iter()
                .any(|c| c.starts_with(&format!("get_health:{}", svc.name)))
        );
    }
}
