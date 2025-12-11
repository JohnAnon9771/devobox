use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerState {
    Running,
    Stopped,
    NotCreated,
}

#[derive(Debug, Clone)]
pub struct Container {
    pub state: ContainerState,
}

impl Container {
    pub fn new(_name: String, state: ContainerState) -> Self {
        Self { state }
    }
}

#[derive(Debug, Clone)]
pub struct ContainerSpec<'a> {
    pub name: &'a str,
    pub image: &'a str,
    pub ports: &'a [String],
    pub env: &'a [String],
    pub network: Option<&'a str>,
    pub userns: Option<&'a str>,
    pub security_opt: Option<&'a str>,
    pub workdir: Option<&'a str>,
    pub volumes: &'a [String],
    pub extra_args: &'a [&'a str],
    pub healthcheck_command: Option<&'a str>,
    pub healthcheck_interval: Option<&'a str>,
    pub healthcheck_timeout: Option<&'a str>,
    pub healthcheck_retries: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    #[default]
    Generic,
    Database,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Service {
    #[serde(default)]
    pub name: String,
    pub image: String,
    #[serde(default, rename = "type")]
    pub kind: ServiceKind,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    pub healthcheck_command: Option<String>,
    pub healthcheck_interval: Option<String>, // e.g., "5s"
    pub healthcheck_timeout: Option<String>,  // e.g., "3s"
    pub healthcheck_retries: Option<u32>,
}

impl Service {
    pub fn to_spec(&self) -> ContainerSpec<'_> {
        ContainerSpec {
            name: &self.name,
            image: &self.image,
            ports: &self.ports,
            env: &self.env,
            volumes: &self.volumes,
            network: None,
            userns: None,
            security_opt: None,
            workdir: None,
            extra_args: &[],
            healthcheck_command: self.healthcheck_command.as_deref(),
            healthcheck_interval: self.healthcheck_interval.as_deref(),
            healthcheck_timeout: self.healthcheck_timeout.as_deref(),
            healthcheck_retries: self.healthcheck_retries,
        }
    }

    /// Create a Service from TOML HashMap entry (name comes from key)
    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}
