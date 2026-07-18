use a4n_service_manager::ServiceManagerError;

pub(super) fn service_manager_error_message(error: &ServiceManagerError) -> String {
    match error {
        ServiceManagerError::UnknownService(service) => {
            format!("unknown service: {service}")
        }
        ServiceManagerError::InvalidConfig(message) => {
            format!("invalid service config: {message}")
        }
        ServiceManagerError::CapabilityDisabled(service) => {
            format!("capability disabled: {service}")
        }
        ServiceManagerError::HealthCheckFailed { service, .. } => {
            format!("service {service} health check failed")
        }
        ServiceManagerError::HttpStatus { service, status } => {
            format!("service {service} returned HTTP status {status}")
        }
        ServiceManagerError::Unhealthy { service, status } => {
            format!("service {service} is unhealthy: {status:?}")
        }
        ServiceManagerError::ProtocolMismatch {
            service,
            expected,
            actual,
        } => {
            format!("service {service} protocol mismatch: expected {expected}, actual {actual}")
        }
        ServiceManagerError::CapabilityMismatch {
            service,
            expected,
            actual,
        } => {
            format!("service {service} capability mismatch: expected {expected}, actual {actual}")
        }
        ServiceManagerError::ServiceIdentityMismatch { expected, actual } => {
            format!("service identity mismatch: expected {expected}, actual {actual}")
        }
        ServiceManagerError::Timeout {
            service,
            timeout_ms,
        } => {
            format!("service {service} startup timed out after {timeout_ms}ms")
        }
        ServiceManagerError::Exited { service, status } => {
            format!("service {service} exited before becoming healthy: status {status:?}")
        }
        ServiceManagerError::ProcessStatusFailed { service, .. } => {
            format!("service {service} process status failed")
        }
        ServiceManagerError::SpawnFailed { service, .. } => {
            format!("service {service} spawn failed")
        }
        ServiceManagerError::StopFailed { service, .. } => {
            format!("service {service} stop failed")
        }
        ServiceManagerError::StartupCleanupFailed {
            service, startup, ..
        } => format!(
            "service {service} startup failed and child cleanup also failed: {}",
            service_manager_error_message(startup.as_ref())
        ),
    }
}
