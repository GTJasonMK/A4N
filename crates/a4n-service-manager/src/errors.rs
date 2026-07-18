use a4n_service_protocol::{HealthStatus, ServiceErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum ServiceManagerError {
    #[error("unknown service: {0}")]
    UnknownService(String),
    #[error("invalid service config: {0}")]
    InvalidConfig(String),
    #[error("capability disabled: {0}")]
    CapabilityDisabled(String),
    #[error("service {service} health check failed: {source}")]
    HealthCheckFailed {
        service: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("service {service} returned HTTP status {status}")]
    HttpStatus { service: String, status: u16 },
    #[error("service {service} is unhealthy: {status:?}")]
    Unhealthy {
        service: String,
        status: HealthStatus,
    },
    #[error("service {service} protocol mismatch: expected {expected}, actual {actual}")]
    ProtocolMismatch {
        service: String,
        expected: u32,
        actual: u32,
    },
    #[error("service identity mismatch: expected {expected}, actual {actual}")]
    ServiceIdentityMismatch { expected: String, actual: String },
    #[error("service {service} capability mismatch: expected {expected}, actual {actual}")]
    CapabilityMismatch {
        service: String,
        expected: String,
        actual: String,
    },
    #[error("service {service} startup timed out after {timeout_ms}ms")]
    Timeout { service: String, timeout_ms: u64 },
    #[error("service {service} exited before becoming healthy: status {status:?}")]
    Exited {
        service: String,
        status: Option<i32>,
    },
    #[error("service {service} process status failed: {source}")]
    ProcessStatusFailed {
        service: String,
        #[source]
        source: std::io::Error,
    },
    #[error("service {service} spawn failed: {source}")]
    SpawnFailed {
        service: String,
        #[source]
        source: std::io::Error,
    },
    #[error("service {service} stop failed: {source}")]
    StopFailed {
        service: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "service {service} startup failed ({startup}); cleanup kill also failed: {kill_source}"
    )]
    StartupCleanupFailed {
        service: String,
        startup: Box<ServiceManagerError>,
        #[source]
        kill_source: std::io::Error,
    },
}

impl ServiceManagerError {
    pub fn protocol_error_code(&self) -> ServiceErrorCode {
        match self {
            Self::CapabilityDisabled(_) => ServiceErrorCode::CapabilityDisabled,
            Self::ProtocolMismatch { .. }
            | Self::ServiceIdentityMismatch { .. }
            | Self::CapabilityMismatch { .. } => ServiceErrorCode::VersionMismatch,
            Self::InvalidConfig(_) => ServiceErrorCode::ConfigurationError,
            _ => ServiceErrorCode::ServiceUnavailable,
        }
    }
}
