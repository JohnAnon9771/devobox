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
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Database {
    pub name: String,
    pub image: String,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
}

impl Database {
    pub fn to_spec(&self) -> ContainerSpec {
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
        }
    }
}
