use crate::domain::{ContainerState, Service, ServiceKind};
use crate::infra::config::{AppConfig, load_app_config, resolve_project_services};
use crate::infra::{PodmanAdapter, ProjectDiscovery};
use crate::services::{
    CleanupOptions, ContainerService, Orchestrator, SystemService, ZellijService,
};
use anyhow::{Context, Result, bail};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

use crate::cli::RuntimeContext;

pub struct Runtime {
    global_config_dir: PathBuf,
    app_config: AppConfig,
    services: Vec<Service>,
    pub container_service: Arc<ContainerService>,
    pub orchestrator: Arc<Orchestrator>,
}

impl Runtime {
    pub fn new(global_config_dir: &Path) -> Result<Self> {
        let runtime = Arc::new(PodmanAdapter::new());
        Self::with_runtime(global_config_dir, runtime)
    }

    pub fn with_runtime(
        global_config_dir: &Path,
        runtime: Arc<dyn crate::domain::ContainerRuntime>,
    ) -> Result<Self> {
        let app_config = load_app_config(global_config_dir)?;

        // Use resolve_all_services to load local services AND dependencies
        let services = crate::infra::config::resolve_all_services(global_config_dir, &app_config)?;

        let container_service = Arc::new(ContainerService::new(runtime.clone()));
        let system_service = Arc::new(SystemService::new(runtime));
        let orchestrator = Arc::new(Orchestrator::new(container_service.clone(), system_service));
        Ok(Self {
            global_config_dir: global_config_dir.to_path_buf(),
            app_config,
            services,
            container_service,
            orchestrator,
        })
    }

    pub fn ensure_dev_container(&self) -> Result<()> {
        self.container_service.ensure_running(
            self.app_config
                .container
                .name
                .as_deref()
                .context("Main container name not set in config")?,
        )
    }

    pub fn start_services_by_filter(&self, kind_filter: Option<ServiceKind>) -> Result<()> {
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

    pub fn stop_services_by_filter(&self, kind_filter: Option<ServiceKind>) -> Result<()> {
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

    pub fn restart_services_by_filter(&self, kind_filter: Option<ServiceKind>) -> Result<()> {
        self.stop_services_by_filter(kind_filter.clone())?;
        self.start_services_by_filter(kind_filter)
    }

    pub fn start_svc(&self, service_name: &str) -> Result<()> {
        let svc = self
            .services
            .iter()
            .find(|s| s.name == service_name)
            .context(format!(
                "Serviço '{}' não está listado na configuração",
                service_name
            ))?;

        self.ensure_svc_created(svc)?;
        self.container_service.start(service_name)
    }

    pub fn stop_svc(&self, service_name: &str) -> Result<()> {
        if !self.is_known_svc(service_name) {
            bail!(
                "Serviço '{}' não está listado na configuração",
                service_name
            );
        }
        self.container_service.stop(service_name)
    }

    pub fn restart_svc(&self, service_name: &str) -> Result<()> {
        if !self.is_known_svc(service_name) {
            bail!(
                "Serviço '{}' não está listado na configuração",
                service_name
            );
        }
        self.container_service.stop(service_name)?;
        self.container_service.start(service_name)
    }

    pub fn is_known_svc(&self, name: &str) -> bool {
        self.services.iter().any(|svc| svc.name == name)
    }

    pub fn status(&self) -> Result<()> {
        println!(" Status dos containers:");
        let mut missing = false;

        for name in self.all_containers() {
            let container = self.container_service.get_status(&name)?;
            let state = match container.state {
                crate::domain::ContainerState::Running => "rodando",
                crate::domain::ContainerState::Stopped => "parado",
                crate::domain::ContainerState::NotCreated => {
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

    pub fn run_shell(&self, with_dbs: bool, auto_stop: bool) -> Result<()> {
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
        let workdir_in_container = container_workdir()?;

        info!(
            " Entrando no {} (workdir {:?})",
            main_container_name, workdir_in_container
        );

        // Check if we are inside a project (devobox.toml exists in current dir)
        let pwd = std::env::current_dir()?;
        let devobox_toml = pwd.join("devobox.toml");

        let home = std::env::var("HOME").unwrap_or_default();
        let code_dir = PathBuf::from(&home).join("code");

        if devobox_toml.exists() {
            let discovery = ProjectDiscovery::new(None)?;
            match discovery.load_project_config(&devobox_toml) {
                Ok(config) => {
                    let project = crate::domain::Project::new(pwd.clone(), config);
                    info!(" Detectado projeto: {}", project.name);

                    let cmd = vec!["devobox", "project", "up", &project.name];

                    let status = std::process::Command::new("podman")
                        .args(["exec", "-it"])
                        .arg(main_container_name)
                        .args(cmd)
                        .status()
                        .context("Falha ao executar devobox project up via podman")?;

                    if !status.success() {
                        bail!("Falha ao iniciar projeto via devobox project up inside container");
                    }

                    if auto_stop {
                        self.stop_all_containers()?;
                    }
                    return Ok(());
                }
                Err(e) => {
                    warn!("Encontrado devobox.toml mas falha ao carregar: {}", e);
                    warn!("Usando shell padrão...");
                }
            }
        }

        let session_name = if let Ok(stripped) = pwd.strip_prefix(&code_dir) {
            if let Some(project_name) = stripped.components().next() {
                // Inside ~/code/project_name but no devobox.toml found
                format!("devobox-{}", project_name.as_os_str().to_string_lossy())
            } else {
                // Inside ~/code root
                "devobox-code-root".to_string()
            }
        } else {
            // Outside ~/code
            "devobox-default".to_string()
        };

        let result = self.container_service.exec_shell(
            main_container_name,
            workdir_in_container.as_deref(),
            Some(&session_name),
        );

        if auto_stop {
            self.stop_all_containers()?;
        }

        result
    }

    pub fn stop_all_containers(&self) -> Result<()> {
        let containers = self.all_containers();
        self.orchestrator.stop_all(&containers)
    }

    pub fn all_containers(&self) -> Vec<String> {
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

    pub fn cleanup(&self, options: &CleanupOptions) -> Result<()> {
        self.orchestrator.cleanup(options)
    }

    pub fn nuke(&self) -> Result<()> {
        self.orchestrator.nuke_system()
    }

    pub fn reset(&self) -> Result<()> {
        self.orchestrator.reset_system()
    }

    pub fn ensure_svc_created(&self, svc: &Service) -> Result<()> {
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

pub fn smart_start(
    config_dir: &Path,
    service: Option<&str>,
    kind: Option<ServiceKind>,
) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;

    if let Some(name) = service {
        if runtime.is_known_svc(name) {
            runtime.start_svc(name)
        } else {
            // Check if it matches the main container name
            let main_name = runtime
                .app_config
                .container
                .name
                .as_deref()
                .unwrap_or("devobox");
            if name == main_name {
                runtime.ensure_dev_container()
            } else {
                bail!("Serviço ou container '{}' não encontrado.", name);
            }
        }
    } else {
        // Start by filter (or all)
        let start_all = kind.is_none();
        runtime.start_services_by_filter(kind)?;
        // Also ensure main container is running if we are starting "everything" (no kind filter)
        if start_all {
            runtime.ensure_dev_container()?;
        }
        Ok(())
    }
}

pub fn smart_stop(
    config_dir: &Path,
    service: Option<&str>,
    kind: Option<ServiceKind>,
) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;

    if let Some(name) = service {
        if runtime.is_known_svc(name) {
            runtime.stop_svc(name)
        } else {
            let main_name = runtime
                .app_config
                .container
                .name
                .as_deref()
                .unwrap_or("devobox");
            if name == main_name {
                runtime.container_service.stop(main_name)
            } else {
                bail!("Serviço ou container '{}' não encontrado.", name);
            }
        }
    } else {
        match kind {
            Some(k) => runtime.stop_services_by_filter(Some(k)),
            None => runtime.stop_all_containers(),
        }
    }
}

pub fn smart_restart(
    config_dir: &Path,
    service: Option<&str>,
    kind: Option<ServiceKind>,
) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;

    if let Some(name) = service {
        if runtime.is_known_svc(name) {
            runtime.restart_svc(name)
        } else {
            let main_name = runtime
                .app_config
                .container
                .name
                .as_deref()
                .unwrap_or("devobox");
            if name == main_name {
                runtime.container_service.stop(main_name)?;
                runtime.ensure_dev_container()
            } else {
                bail!("Serviço ou container '{}' não encontrado.", name);
            }
        }
    } else {
        match kind {
            Some(k) => runtime.restart_services_by_filter(Some(k)),
            None => {
                runtime.stop_all_containers()?;
                runtime.start_services_by_filter(None)?;
                runtime.ensure_dev_container()
            }
        }
    }
}

#[allow(dead_code)]
pub fn exec_cmd(config_dir: &Path, command: Vec<String>) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;

    // Ensure container is running before exec
    runtime.ensure_dev_container()?;

    let main_container_name = runtime
        .app_config
        .container
        .name
        .as_deref()
        .context("Main container name not set in config")?;

    let workdir_in_container = container_workdir()?;

    // Construct the podman exec command
    let mut args = vec!["exec".to_string(), "-it".to_string()];
    if let Some(wd) = workdir_in_container {
        args.push("-w".to_string());
        args.push(wd.to_string_lossy().to_string());
    }
    args.push(main_container_name.to_string());
    args.extend(command);

    let status = std::process::Command::new("podman")
        .args(&args)
        .status()
        .context("Falha ao executar comando via podman exec")?;

    if !status.success() {
        bail!("Comando falhou com status: {:?}", status);
    }
    Ok(())
}

pub fn cleanup(config_dir: &Path, options: &CleanupOptions) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.cleanup(options)
}

pub fn nuke(config_dir: &Path) -> Result<()> {
    let runtime = Runtime::new(config_dir)?;
    runtime.nuke()
}

pub fn reset(config_dir: &Path) -> Result<()> {
    warn!(" System reset irá DELETAR TUDO do Podman!");
    warn!("   Esta ação é IRREVERSÍVEL!");
    info!("");
    info!(" Digite 'RESET' para confirmar:");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim() != "RESET" {
        info!(" Reset cancelado.");
        return Ok(());
    }

    let runtime = Runtime::new(config_dir)?;
    runtime.reset()?;

    info!("");
    warn!("  Você precisará rodar 'devobox init' novamente.");
    Ok(())
}

/// Lists all available projects
pub fn project_list(_config_dir: &Path) -> Result<()> {
    let discovery = ProjectDiscovery::new(None)?;
    let projects = discovery.discover_all()?;

    if projects.is_empty() {
        info!(" Nenhum projeto encontrado em ~/code");
        info!(" Dica: Crie um diretório com devobox.toml para começar");
        info!("");
        info!(" Exemplo:");
        info!("   mkdir -p ~/code/meu-projeto");
        info!("   cd ~/code/meu-projeto");
        info!("   echo '[project]' > devobox.toml");
        return Ok(());
    }

    info!(" Projetos disponíveis:");
    for project in projects {
        let services_info = if project.config.services.is_some()
            && !project.config.services.as_ref().unwrap().is_empty()
        {
            " (com serviços configurados)"
        } else {
            ""
        };
        info!("  - {}{}", project.name, services_info);
    }

    Ok(())
}

/// Activates a project workspace (container context only)
pub fn project_up(config_dir: &Path, project_name: &str) -> Result<()> {
    let context = RuntimeContext::detect();

    if context.is_host() {
        bail!(
            "'devobox project up' só funciona dentro do container.\nUse 'devobox' ou 'devobox shell' primeiro."
        );
    }

    // 1. Find project
    let discovery = ProjectDiscovery::new(None)?;
    let project = discovery
        .find_project(project_name)?
        .with_context(|| format!("Projeto '{}' não encontrado em ~/code", project_name))?;

    info!(" Ativando projeto: {}", project.name);

    // 2. Load and start project-specific services
    let services = resolve_project_services(&project, config_dir)?;

    if !services.is_empty() {
        info!(" Iniciando {} serviço(s)...", services.len());

        // Create Runtime to access orchestrator
        let runtime = Runtime::new(config_dir)?;

        // Ensure all services are created first
        for svc in &services {
            if let Err(e) = runtime.ensure_svc_created(svc) {
                warn!("  Aviso ao criar serviço {}: {}", svc.name, e);
            }
        }

        // Start all services
        if let Err(e) = runtime.orchestrator.start_all(&services) {
            warn!("  Erro ao iniciar serviços: {}", e);
            warn!("  Continuando mesmo assim...");
        } else {
            info!(" Serviços iniciados com sucesso!");
        }
    }

    // 3. Log environment variables (can't actually set them for parent shell)
    if !project.env_vars().is_empty() {
        info!(" Variáveis de ambiente do projeto:");
        for env_var in project.env_vars() {
            info!("   {}", env_var);
        }
    }

    // 4. Gather dependent projects info for layout
    let mut dependencies_info =
        Vec::with_capacity(project.config.dependencies.include_projects.len());
    if !project.config.dependencies.include_projects.is_empty() {
        for relative_path in &project.config.dependencies.include_projects {
            let dep_path = project.path.join(relative_path);
            let canonical_path = match std::fs::canonicalize(&dep_path) {
                Ok(p) => p,
                Err(_) => {
                    warn!("  Caminho de dependência inválido: {:?}", dep_path);
                    continue;
                }
            };

            // Try to load project config to get startup_command
            let config_path = canonical_path.join("devobox.toml");
            let startup_command = if config_path.exists() {
                match std::fs::read_to_string(&config_path) {
                    Ok(content) => match toml::from_str::<crate::domain::ProjectConfig>(&content) {
                        Ok(cfg) => cfg.project.and_then(|p| p.startup_command),
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            } else {
                None
            };

            let name = canonical_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            dependencies_info.push(crate::services::ProjectLayoutInfo {
                name,
                path: canonical_path,
                startup_command,
            });
        }
    }

    // 5. Create/attach Zellij session
    let zellij = ZellijService::new();
    let session_name = project.session_name();

    info!(" Abrindo sessão Zellij: {}", session_name);
    info!(" Diretório de trabalho: {}", project.path.display());
    if !dependencies_info.is_empty() {
        info!(" Projetos incluídos no layout: {}", dependencies_info.len());
    }

    zellij.create_with_layout(
        &session_name,
        &crate::services::ProjectLayoutInfo {
            name: project.name.clone(),
            path: project.path.clone(),
            startup_command: project.startup_command().map(String::from),
        },
        &dependencies_info,
    )?;

    Ok(())
}

/// Shows current project info
pub fn project_info() -> Result<()> {
    let context = RuntimeContext::detect();

    info!(" Contexto: {}", context);

    if context.is_host() {
        info!(" Você está rodando no host (fora do container)");
        return Ok(());
    }

    // Try to detect current project from PWD
    let pwd = env::current_dir()?;
    let home = env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
    let code_dir = PathBuf::from(&home).join("code");

    if let Ok(stripped) = pwd.strip_prefix(&code_dir) {
        if let Some(project_name) = stripped.components().next() {
            info!(
                " Projeto atual: {}",
                project_name.as_os_str().to_string_lossy()
            );
        } else {
            info!(" Projeto atual: (raiz de ~/code)");
        }
    } else {
        info!(" Projeto atual: (nenhum - fora de ~/code)");
    }

    info!(" Diretório: {}", pwd.display());

    // Show active Zellij sessions
    let zellij = ZellijService::new();
    if zellij.is_available() {
        match zellij.list_sessions() {
            Ok(sessions) if !sessions.is_empty() => {
                info!("");
                info!(" Sessões Zellij ativas:");
                for session in sessions {
                    info!("   - {}", session);
                }
            }
            Ok(_) => {
                info!("");
                info!(" Nenhuma sessão Zellij ativa");
            }
            Err(e) => {
                warn!("  Erro ao listar sessões Zellij: {}", e);
            }
        }
    } else {
        info!("");
        info!(" Zellij não está instalado");
        info!(" Instale com: mise install zellij");
    }

    Ok(())
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
