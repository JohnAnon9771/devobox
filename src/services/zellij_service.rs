use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{info, warn};

const DEFAULT_LAYOUT_TEMPLATE: &str = r#"layout {
    default_tab_template {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        children
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
"#;

pub struct ProjectLayoutInfo {
    pub name: String,
    pub path: PathBuf,
    pub startup_command: Option<String>,
}

/// Manages Zellij sessions for projects
pub struct ZellijService;

impl ZellijService {
    pub fn new() -> Self {
        Self
    }

    /// Checks if Zellij is installed and available in PATH
    pub fn is_available(&self) -> bool {
        Command::new("which")
            .arg("zellij")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Creates or attaches to a Zellij session with a generated layout
    pub fn create_with_layout(
        &self,
        session_name: &str,
        main_project: &ProjectLayoutInfo,
        dependencies: &[ProjectLayoutInfo],
    ) -> Result<()> {
        if !self.is_available() {
            bail!(
                "Zellij não está instalado.\n\
                 Instale com: mise install zellij\n\
                 Ou adicione ao mise.toml: zellij = \"latest\""
            );
        }

        // Check if session exists
        let exists = self.session_exists(session_name)?;

        if exists {
            info!("  Anexando à sessão existente: {}", session_name);
            self.attach(session_name)
        } else {
            info!("  Criando nova sessão com layout: {}", session_name);
            let layout_path =
                self.generate_layout_file(session_name, main_project, dependencies)?;
            let res = self.create_with_layout_file(session_name, &main_project.path, &layout_path);

            // Cleanup temp file
            if let Err(e) = std::fs::remove_file(&layout_path) {
                warn!(
                    "Não foi possível remover arquivo temporário de layout: {}",
                    e
                );
            }

            res
        }
    }

    /// Generates a temporary KDL layout file
    fn generate_layout_file(
        &self,
        session_name: &str,
        main_project: &ProjectLayoutInfo,
        dependencies: &[ProjectLayoutInfo],
    ) -> Result<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("{}.kdl", session_name));
        let mut file = File::create(&file_path)?;

        // Write the static header (Tab bar and Status bar configuration)
        file.write_all(DEFAULT_LAYOUT_TEMPLATE.as_bytes())?;

        // Write Main Project Tab (Focused)
        self.write_project_tab(&mut file, main_project, true)?;

        // Write Dependency Tabs
        for dep in dependencies {
            self.write_project_tab(&mut file, dep, false)?;
        }

        writeln!(file, "}}")?; // Close layout

        Ok(file_path)
    }

    /// Helper to write a single project tab configuration
    fn write_project_tab(
        &self,
        file: &mut File,
        project: &ProjectLayoutInfo,
        focus: bool,
    ) -> Result<()> {
        writeln!(
            file,
            "    tab name=\"{}\" {} {{",
            project.name,
            if focus { "focus=true" } else { "" }
        )?;

        writeln!(
            file,
            "        pane cwd=\"{}\" {{",
            project.path.to_string_lossy()
        )?;

        if let Some(cmd) = &project.startup_command {
            // Split command string into program and args
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if !parts.is_empty() {
                writeln!(file, "            command \"{}\"", parts[0])?;
                if parts.len() > 1 {
                    let args = parts[1..]
                        .iter()
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(" ");
                    writeln!(file, "            args {}", args)?;
                }
            }
        }

        writeln!(file, "        }}")?; // Close pane
        writeln!(file, "    }}")?; // Close tab
        Ok(())
    }

    fn create_with_layout_file(
        &self,
        session_name: &str,
        workdir: &Path,
        layout_path: &Path,
    ) -> Result<()> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
        let config_dir = Path::new(&home).join(".config/zellij");

        let status = Command::new("zellij")
            .args([
                "attach",
                "--create",
                session_name,
                "--layout",
                &layout_path.to_string_lossy(),
            ])
            .env("ZELLIJ_CONFIG_DIR", config_dir)
            .current_dir(workdir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Falha ao criar sessão do Zellij com layout")?;

        if !status.success() {
            bail!("Criação de sessão do Zellij falhou");
        }

        Ok(())
    }

    /// Creates or attaches to a Zellij session
    ///
    /// If the session already exists, attaches to it.
    /// Otherwise, creates a new session with the given name.
    ///
    /// # Arguments
    /// * `session_name` - Name of the Zellij session
    /// * `workdir` - Working directory for the session
    ///
    /// # Returns
    /// * `Ok(())` - Session was created/attached successfully
    /// * `Err` - If Zellij is not installed or session creation failed
    pub fn attach_or_create(&self, session_name: &str, workdir: &Path) -> Result<()> {
        if !self.is_available() {
            bail!(
                "Zellij não está instalado.\n\
                 Instale com: mise install zellij\n\
                 Ou adicione ao mise.toml: zellij = \"latest\""
            );
        }

        // Check if session exists
        let exists = self.session_exists(session_name)?;

        if exists {
            info!("  Anexando à sessão existente: {}", session_name);
            self.attach(session_name)
        } else {
            info!("  Criando nova sessão: {}", session_name);
            self.create(session_name, workdir)
        }
    }

    /// Checks if a session exists
    ///
    /// # Arguments
    /// * `session_name` - Name of the session to check
    ///
    /// # Returns
    /// * `Ok(true)` - Session exists
    /// * `Ok(false)` - Session doesn't exist
    /// * `Err` - Error checking sessions
    fn session_exists(&self, session_name: &str) -> Result<bool> {
        let output = Command::new("zellij")
            .args(["list-sessions"])
            .output()
            .context("Falha ao listar sessões do Zellij")?;

        let sessions = String::from_utf8_lossy(&output.stdout);
        Ok(sessions.lines().any(|line| line.contains(session_name)))
    }

    /// Creates a new session
    ///
    /// # Arguments
    /// * `session_name` - Name for the new session
    /// * `workdir` - Working directory for the session
    ///
    /// # Returns
    /// * `Ok(())` - Session created successfully
    /// * `Err` - Session creation failed
    fn create(&self, session_name: &str, workdir: &Path) -> Result<()> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
        let config_dir = Path::new(&home).join(".config/zellij");

        let status = Command::new("zellij")
            .args(["attach", "--create", session_name])
            .env("ZELLIJ_CONFIG_DIR", config_dir)
            .current_dir(workdir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Falha ao criar sessão do Zellij")?;

        if !status.success() {
            bail!("Criação de sessão do Zellij falhou");
        }

        Ok(())
    }

    /// Attaches to existing session
    ///
    /// # Arguments
    /// * `session_name` - Name of the session to attach to
    ///
    /// # Returns
    /// * `Ok(())` - Attached successfully
    /// * `Err` - Attach failed
    fn attach(&self, session_name: &str) -> Result<()> {
        let status = Command::new("zellij")
            .args(["attach", session_name])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Falha ao anexar à sessão do Zellij")?;

        if !status.success() {
            bail!("Anexar à sessão do Zellij falhou");
        }

        Ok(())
    }

    /// Lists all active Zellij sessions
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of session names
    /// * `Err` - Failed to list sessions
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        if !self.is_available() {
            return Ok(Vec::new());
        }

        let output = Command::new("zellij")
            .args(["list-sessions"])
            .output()
            .context("Falha ao listar sessões")?;

        let sessions = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(sessions)
    }

    /// Kills a specific session
    ///
    /// # Arguments
    /// * `session_name` - Name of the session to kill
    ///
    /// # Returns
    /// * `Ok(())` - Session killed successfully
    /// * `Err` - Failed to kill session
    pub fn kill_session(&self, session_name: &str) -> Result<()> {
        if !self.is_available() {
            bail!("Zellij não está instalado");
        }

        let status = Command::new("zellij")
            .args(["delete-session", session_name])
            .status()
            .context(format!("Falha ao matar sessão: {}", session_name))?;

        if !status.success() {
            warn!("Falha ao deletar sessão {}", session_name);
        }

        Ok(())
    }
}

impl Default for ZellijService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zellij_service_creation() {
        let _service = ZellijService::new();
        // Just verify it can be created without panicking
    }

    #[test]
    fn test_is_available_doesnt_panic() {
        let service = ZellijService::new();
        // This should not panic, whether zellij is installed or not
        let _available = service.is_available();
    }

    #[test]
    fn test_list_sessions_when_not_available() {
        let service = ZellijService::new();
        if !service.is_available() {
            let sessions = service.list_sessions().unwrap();
            assert!(sessions.is_empty());
        }
    }

    #[test]
    fn test_generate_layout_file_structure() {
        let service = ZellijService::new();

        let main_project = ProjectLayoutInfo {
            name: "main-app".to_string(),
            path: PathBuf::from("/code/main"),
            startup_command: Some("npm start".to_string()),
        };

        let deps = vec![
            ProjectLayoutInfo {
                name: "api-service".to_string(),
                path: PathBuf::from("/code/api"),
                startup_command: Some("cargo run --release".to_string()),
            },
            ProjectLayoutInfo {
                name: "db-service".to_string(),
                path: PathBuf::from("/code/db"),
                startup_command: None,
            },
        ];

        let layout_path = service
            .generate_layout_file("test-session", &main_project, &deps)
            .expect("Failed to generate layout file");

        // Read the file content
        let content =
            std::fs::read_to_string(&layout_path).expect("Failed to read generated layout file");

        // Cleanup immediately
        let _ = std::fs::remove_file(layout_path);

        // Verify structure
        assert!(content.contains("layout {"));
        assert!(content.contains("default_tab_template {"));

        // Verify Main Project
        assert!(content.contains("tab name=\"main-app\" focus=true {"));
        assert!(content.contains("pane cwd=\"/code/main\" {"));
        assert!(content.contains("command \"npm\""));
        assert!(content.contains("args \"start\""));

        // Verify Dependency 1
        assert!(content.contains("tab name=\"api-service\"  {")); // Note: extra space might happen due to empty focus string
        assert!(content.contains("pane cwd=\"/code/api\" {"));
        assert!(content.contains("command \"cargo\""));
        assert!(content.contains("args \"run\" \"--release\""));

        // Verify Dependency 2 (No command)
        assert!(content.contains("tab name=\"db-service\"  {"));
        assert!(content.contains("pane cwd=\"/code/db\" {"));
        assert!(!content.contains("command \"/code/db\"")); // Shouldn't treat path as command
    }
}
