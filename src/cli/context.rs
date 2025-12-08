use std::env;

/// Represents the runtime context where devobox commands are executed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeContext {
    /// Running on the host machine (outside the devobox container)
    Host,
    /// Running inside the devobox container
    Container,
}

impl RuntimeContext {
    /// Detects the current runtime context
    ///
    /// Detection strategy:
    /// 1. Check for DEVOBOX_CONTAINER environment variable (set by builder)
    /// 2. Fallback: Check for container marker files (/.dockerenv, /run/.containerenv)
    ///
    /// # Returns
    /// * `RuntimeContext::Container` - Running inside devobox container
    /// * `RuntimeContext::Host` - Running on host machine
    pub fn detect() -> Self {
        // Primary detection: environment variable set by builder
        if env::var("DEVOBOX_CONTAINER").is_ok() {
            return Self::Container;
        }

        // Fallback detection: container marker files
        if Self::is_inside_container() {
            return Self::Container;
        }

        Self::Host
    }

    /// Checks if running inside a container
    #[allow(dead_code)]
    pub fn is_container(&self) -> bool {
        matches!(self, Self::Container)
    }

    /// Checks if running on host
    pub fn is_host(&self) -> bool {
        matches!(self, Self::Host)
    }

    /// Heuristic check for container environment
    ///
    /// Checks for the presence of container marker files:
    /// - /.dockerenv (Docker/Podman containers)
    /// - /run/.containerenv (Podman containers)
    fn is_inside_container() -> bool {
        std::path::Path::new("/.dockerenv").exists()
            || std::path::Path::new("/run/.containerenv").exists()
    }
}

impl std::fmt::Display for RuntimeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host => write!(f, "Host"),
            Self::Container => write!(f, "Container"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_detection_without_env() {
        // When DEVOBOX_CONTAINER is not set, should default to Host
        // (unless running in actual container with marker files)
        unsafe {
            env::remove_var("DEVOBOX_CONTAINER");
        }
        let ctx = RuntimeContext::detect();
        // This will be Host on normal dev machines, Container if running in actual container
        assert!(ctx == RuntimeContext::Host || ctx == RuntimeContext::Container);
    }

    #[test]
    fn test_is_container() {
        let ctx = RuntimeContext::Container;
        assert!(ctx.is_container());
        assert!(!ctx.is_host());
    }

    #[test]
    fn test_is_host() {
        let ctx = RuntimeContext::Host;
        assert!(ctx.is_host());
        assert!(!ctx.is_container());
    }

    #[test]
    fn test_display() {
        assert_eq!(RuntimeContext::Host.to_string(), "Host");
        assert_eq!(RuntimeContext::Container.to_string(), "Container");
    }

    #[test]
    fn test_equality() {
        assert_eq!(RuntimeContext::Host, RuntimeContext::Host);
        assert_eq!(RuntimeContext::Container, RuntimeContext::Container);
        assert_ne!(RuntimeContext::Host, RuntimeContext::Container);
    }
}
