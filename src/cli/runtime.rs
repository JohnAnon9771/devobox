use anyhow::{Context, Result, bail};
use devobox::domain::{ContainerState, Service, ServiceKind};
use devobox::infra::PodmanAdapter;
use devobox::infra::config::{AppConfig, load_app_config};
use devobox::services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

struct Runtime {
    global_config_dir: PathBuf,
    app_config: AppConfig,
    services: Vec<Service>,
    container_service: Arc<ContainerService>,
    orchestrator: Orchestrator,
}

impl Runtime {
    fn new(global_config_dir: &Path) -> Result<Self> {
        let app_config = load_app_config(global_config_dir)?;

        // Use resolve_all_services to load local services AND dependencies
        let services =
            devobox::infra::config::resolve_all_services(global_config_dir, &app_config)?;

        let runtime = Arc::new(PodmanAdapter::new());
        let container_service = Arc::new(ContainerService::new(runtime.clone()));
        let system_service = Arc::new(SystemService::new(runtime));
        let orchestrator = Orchestrator::new(container_service.clone(), system_service);
        Ok(Self {
            global_config_dir: global_config_dir.to_path_buf(),
            app_config,
            services,
            container_service,
            orchestrator,
        })
    }

    fn ensure_dev_container(&self) -> Result<()> {
        self.container_service.ensure_running(
            self.app_config
                .container
                .name
                .as_deref()
                .context("Main container name not set in config")?,
        )
    }

    fn start_services_by_filter(&self, kind_filter: Option<ServiceKind>) -> Result<()> {
        if self.services.is_empty() {
            warn!(
                "  Nenhum serviço configurado em {:?}",
                self.global_config_dir
            );
            return Ok(());
        }

        let services_to_start: Vec<&Service> = match kind_filter {
            Some(k) => self.services.iter().filter(|s| s.kind == k).collect(),
            None => self.services.iter().collect(),
        };

        if services_to_start.is_empty() {
            return Ok(());
        }

        // ensure services are created before starting
        for svc in &services_to_start {
            self.ensure_svc_created(svc)?;
        }

        let svc_names: Vec<Service> = services_to_start.into_iter().cloned().collect();
        self.orchestrator.start_all(&svc_names)
    }

    fn stop_services_by_filter(&self, kind_filter: Option<ServiceKind>) -> Result<()> {
        if self.services.is_empty() {
            return Ok(());
        }

        let services_to_stop: Vec<&Service> = match kind_filter {
            Some(k) => self.services.iter().filter(|s| s.kind == k).collect(),
            None => self.services.iter().collect(),
        };

        if services_to_stop.is_empty() {
            return Ok(());
        }

        let svc_names: Vec<String> = services_to_stop
            .iter()
            .map(|svc| svc.name.clone())
            .collect();
        self.orchestrator.stop_all(&svc_names)
    }

    fn restart_services_by_filter(&self, kind_filter: Option<ServiceKind>) -> Result<()> {
        self.stop_services_by_filter(kind_filter.clone())?;
        self.start_services_by_filter(kind_filter)
    }

    fn start_svc(&self, service_name: &str) -> Result<()> {
        let svc = self
            .services
            .iter()
            .find(|s| s.name == service_name)
            .context(format!(
                "Serviço '{service_name}' não está listado em services.yml"
            ))?;

        self.ensure_svc_created(svc)?;
        self.container_service.start(service_name)
    }

    fn stop_svc(&self, service_name: &str) -> Result<()> {
        if !self.is_known_svc(service_name) {
            bail!("Serviço '{service_name}' não está listado em services.yml");
        }
        self.container_service.stop(service_name)
    }

    fn restart_svc(&self, service_name: &str) -> Result<()> {
        if !self.is_known_svc(service_name) {
            bail!("Serviço '{service_name}' não está listado em services.yml");
        }
        self.container_service.stop(service_name)?;
        self.container_service.start(service_name)
    }

    fn is_known_svc(&self, name: &str) -> bool {
        self.services.iter().any(|svc| svc.name == name)
    }

    fn status(&self) -> Result<()> {
        println!(" Status dos containers:");
        let mut missing = false;

        for name in self.all_containers() {
            let container = self.container_service.get_status(&name)?;
            let state = match container.state {
                devobox::domain::ContainerState::Running => "rodando",
                devobox::domain::ContainerState::Stopped => "parado",
                devobox::domain::ContainerState::NotCreated => {
                    missing = true;
                    "não criado"
                }
            };

            println!("- {:<10} | {}", name, state);
        }

        if missing {
            warn!("  Há containers ausentes. Rode 'devobox builder build'.");
        }

        Ok(())
    }

    fn run_shell(&self, with_dbs: bool, auto_stop: bool) -> Result<()> {
        if with_dbs {
            self.start_services_by_filter(None)?;
        }

        self.ensure_dev_container()?;

        let main_container_name = self
            .app_config
            .container
            .name
            .as_deref()
            .context("Main container name not set in config")?;
        let workdir_in_container = container_workdir()?; // This returns a path *inside* the container

        info!(
            " Entrando no {} (workdir {:?})",
            main_container_name, workdir_in_container
        );
        let result = self
            .container_service
            .exec_shell(main_container_name, workdir_in_container.as_deref());

        // Stop all containers on exit if auto_stop is enabled
        if auto_stop {
            self.stop_all_containers()?;
        }

        result
    }

    fn stop_all_containers(&self) -> Result<()> {
        let containers = self.all_containers();
        self.orchestrator.stop_all(&containers)
    }

    fn all_containers(&self) -> Vec<String> {
        let mut names = Vec::with_capacity(self.services.len() + 1);
        names.push(
            self.app_config
                .container
                .name
                .clone()
                .context("Main container name not set in config")
                .expect("Failed to get main container name from config")
                .clone(),
        );
        names.extend(self.services.iter().map(|svc| svc.name.clone()));
        names
    }

    fn cleanup(&self, options: &CleanupOptions) -> Result<()> {
        self.orchestrator.cleanup(options)
    }

    fn nuke(&self) -> Result<()> {
        self.orchestrator.nuke_system()
    }

    fn ensure_svc_created(&self, svc: &Service) -> Result<()> {
        let status = self.container_service.get_status(&svc.name)?;

        if status.state == ContainerState::NotCreated {
            info!(" Criando container para {}...", svc.name);
            self.container_service.recreate(&svc.to_spec())?;
        }

        Ok(())
    }
}

pub fn shell(config_dir: &Path, with_dbs: bool, auto_stop: bool) -> Result<()> {
    if !config_dir.exists() {
        warn!("  Ambiente não configurado.");
        info!(" Executando setup inicial automaticamente...\n");

        crate::cli::setup::install(config_dir)?;
    }

    let runtime = Runtime::new(config_dir)?;

    let main_container_name = runtime
        .app_config
        .container
        .name
        .as_deref()
        .context("Main container name not set in config")?;
    let devobox_status = runtime.container_service.get_status(main_container_name)?;
    if devobox_status.state == ContainerState::NotCreated {
        warn!("  Container '{}' não encontrado.", main_container_name);
        info!(" Construindo ambiente...\n");

        crate::cli::builder::build(config_dir, false)?;
    }

    info!("\n Ambiente pronto! Abrindo shell...\n");

    runtime.run_shell(with_dbs, auto_stop)
}

pub fn up(config_dir: &Path, dbs_only: bool, services_only: bool) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;

    if dbs_only {
        runtime.start_services_by_filter(Some(ServiceKind::Database))?;
    } else if services_only {
        runtime.start_services_by_filter(Some(ServiceKind::Generic))?;
    } else {
        runtime.start_services_by_filter(None)?;
    }

    runtime.ensure_dev_container()
}

pub fn down(config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    for name in runtime.all_containers() {
        runtime.container_service.stop(&name)?;
    }
    info!(" Tudo parado");
    Ok(())
}

pub fn status(config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.status()
}

pub fn svc_start(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.start_svc(name),
        None => runtime.start_services_by_filter(Some(ServiceKind::Generic)),
    }
}

pub fn svc_stop(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.stop_svc(name),
        None => runtime.stop_services_by_filter(Some(ServiceKind::Generic)),
    }
}

pub fn svc_restart(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.restart_svc(name),
        None => runtime.restart_services_by_filter(Some(ServiceKind::Generic)),
    }
}

pub fn db_start(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.start_svc(name),
        None => runtime.start_services_by_filter(Some(ServiceKind::Database)),
    }
}

pub fn db_stop(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.stop_svc(name),
        None => runtime.stop_services_by_filter(Some(ServiceKind::Database)),
    }
}

pub fn db_restart(config_dir: &Path, service: Option<&str>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    match service {
        Some(name) => runtime.restart_svc(name),
        None => runtime.restart_services_by_filter(Some(ServiceKind::Database)),
    }
}

pub fn cleanup(config_dir: &Path, options: &CleanupOptions) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.cleanup(options)
}

pub fn nuke(config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.nuke()
}

fn container_workdir() -> Result<Option<PathBuf>> {
    let pwd = std::env::current_dir()?;
    let home = std::env::var("HOME").unwrap_or_default();
    let home_path = PathBuf::from(&home);
    let code_dir = home_path.join("code");

    if let Ok(stripped) = pwd.strip_prefix(&code_dir) {
        return Ok(Some(PathBuf::from("/home/dev/code").join(stripped)));
    }

    Ok(Some(PathBuf::from("/home/dev")))
}
