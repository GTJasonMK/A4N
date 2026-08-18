use super::contracts::{HealthStatus, ServiceHealth};
use super::errors::ServiceHealthValidationError;
use crate::registry::PROTOCOL_VERSION;

pub fn validate_service_health_envelope(
    health: &ServiceHealth,
    expected_service_name: &str,
    expected_capability_version: Option<&str>,
) -> Result<(), ServiceHealthValidationError> {
    if health.service != expected_service_name {
        return Err(ServiceHealthValidationError::ServiceIdentityMismatch {
            expected: expected_service_name.to_string(),
            actual: health.service.clone(),
        });
    }

    if health.status != HealthStatus::Healthy {
        return Err(ServiceHealthValidationError::Unhealthy {
            service: health.service.clone(),
            status: health.status,
        });
    }

    if health.version.protocol_version != PROTOCOL_VERSION {
        return Err(ServiceHealthValidationError::ProtocolMismatch {
            service: health.service.clone(),
            expected: PROTOCOL_VERSION,
            actual: health.version.protocol_version,
        });
    }

    if let Some(expected) = expected_capability_version
        && health.version.capability_version != expected
    {
        return Err(ServiceHealthValidationError::CapabilityMismatch {
            service: health.service.clone(),
            expected: expected.to_string(),
            actual: health.version.capability_version.clone(),
        });
    }

    Ok(())
}
