use super::contracts::{HealthStatus, ServiceHealth};
use super::errors::ServiceHealthValidationError;
use crate::compatibility::{VersionNegotiationError, capability_major, validate_service_version};
use crate::registry::PROTOCOL_VERSION;

pub fn validate_service_health_envelope(
    health: &ServiceHealth,
    expected_service_name: Option<&str>,
    expected_capability_version: Option<&str>,
) -> Result<(), ServiceHealthValidationError> {
    if let Some(expected) = expected_service_name
        && health.service != expected
    {
        return Err(ServiceHealthValidationError::ServiceIdentityMismatch {
            expected: expected.to_string(),
            actual: health.service.clone(),
        });
    }

    if health.status != HealthStatus::Healthy {
        return Err(ServiceHealthValidationError::Unhealthy {
            service: health.service.clone(),
            status: health.status,
        });
    }

    let expected_service = expected_service_name.unwrap_or(health.service.as_str());
    validate_service_version(
        expected_service,
        PROTOCOL_VERSION,
        health.version.protocol_version,
        expected_capability_version,
        &health.version.capability_version,
    )
    .map_err(|error| match error {
        VersionNegotiationError::ProtocolOutOfRange { .. } => {
            ServiceHealthValidationError::ProtocolMismatch {
                service: health.service.clone(),
                expected: PROTOCOL_VERSION,
                actual: health.version.protocol_version,
            }
        }
        VersionNegotiationError::CapabilityOutOfRange {
            expected, actual, ..
        } => ServiceHealthValidationError::CapabilityMismatch {
            service: health.service.clone(),
            expected,
            actual,
        },
    })?;

    if let Some(advertised) = health.version.compatibility {
        if !advertised.protocol.contains(PROTOCOL_VERSION)
            || !advertised
                .protocol
                .contains(health.version.protocol_version)
        {
            return Err(ServiceHealthValidationError::ProtocolMismatch {
                service: health.service.clone(),
                expected: advertised.protocol.max,
                actual: health.version.protocol_version,
            });
        }
        if let Some(expected) = expected_capability_version
            && capability_major(&health.version.capability_version)
                .is_none_or(|major| !advertised.capability_major.contains(major))
        {
            return Err(ServiceHealthValidationError::CapabilityMismatch {
                service: health.service.clone(),
                expected: expected.to_string(),
                actual: health.version.capability_version.clone(),
            });
        }
    }

    Ok(())
}
