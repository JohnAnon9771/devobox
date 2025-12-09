use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{info, warn};

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
        let status = Command::new("zellij")
            .args(["attach", "--create", session_name])
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
}
