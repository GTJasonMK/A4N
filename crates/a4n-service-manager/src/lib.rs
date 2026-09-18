//! Sidecar service configuration, health checks, and owned process startup.

mod config;
mod errors;
mod health;
mod integrity;
mod manager;
mod process;
#[cfg(test)]
mod tests;

pub use a4n_process_catalog::{MigrationCompatibility, ProcessKind, ProcessMetadata};
pub use config::{
    ManagedServiceConfig, RemoteSecurityConfig, RestartPolicy, ServiceMode, is_loopback_http_url,
    normalize_remote_service_url,
};
pub use errors::ServiceManagerError;
pub use manager::ServiceManager;
pub use process::{ManagedSidecarProcess, ManagedSidecarState, ManagedSidecarStatus};
