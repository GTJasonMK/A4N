//! Sidecar service configuration, health checks, and owned process startup.

mod config;
mod errors;
mod health;
mod manager;
mod process;
#[cfg(test)]
mod tests;

pub use config::{ManagedServiceConfig, ServiceMode};
pub use errors::ServiceManagerError;
pub use manager::ServiceManager;
pub use process::ManagedSidecarProcess;
