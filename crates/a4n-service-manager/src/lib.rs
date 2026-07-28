//! Sidecar service configuration, health checks, and owned process startup.

mod config;
mod errors;
mod health;
mod integrity;
mod manager;
mod process;
#[cfg(test)]
mod tests;

pub use config::{ManagedServiceConfig, RemoteSecurityConfig, RestartPolicy, ServiceMode};
pub use errors::ServiceManagerError;
pub use manager::{HttpClientFactory, ServiceManager};
pub use process::{ManagedSidecarProcess, ManagedSidecarState, ManagedSidecarStatus};
