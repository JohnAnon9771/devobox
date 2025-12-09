pub mod cli;
pub mod domain;
pub mod infra;
pub mod services;

// Make test_support available for integration tests
// In a real production crate, we might use a feature flag "test-utils"
pub mod test_support;

pub use domain::{
    Container, ContainerRuntime, ContainerSpec, ContainerState, Service, ServiceKind,
};
pub use infra::PodmanAdapter;
pub use services::{CleanupOptions, ContainerService, Orchestrator, SystemService};
