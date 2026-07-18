use super::config::ManagedServiceConfig;
use super::errors::ServiceManagerError;
use a4n_service_protocol::{
    ServiceHealth, ServiceHealthValidationError, expected_capability_version_for_service,
    validate_service_health_envelope,
};

pub(super) async fn http_health(
    client: &reqwest::Client,
    config: &ManagedServiceConfig,
) -> Result<ServiceHealth, ServiceManagerError> {
    let url = config.health_url()?;
    let response =
        client
            .get(&url)
            .send()
            .await
            .map_err(|err| ServiceManagerError::HealthCheckFailed {
                service: config.name.clone(),
                source: err,
            })?;
    if !response.status().is_success() {
        return Err(ServiceManagerError::HttpStatus {
            service: config.name.clone(),
            status: response.status().as_u16(),
        });
    }
    let health = response.json::<ServiceHealth>().await.map_err(|err| {
        ServiceManagerError::HealthCheckFailed {
            service: config.name.clone(),
            source: err,
        }
    })?;
    validate_service_health(config, &health)?;
    Ok(health)
}

pub(super) fn validate_service_health(
    config: &ManagedServiceConfig,
    health: &ServiceHealth,
) -> Result<(), ServiceManagerError> {
    validate_service_health_envelope(
        health,
        Some(config.name.as_str()),
        expected_capability_version(&config.name),
    )
    .map_err(map_health_validation_error)
}

pub(super) fn expected_capability_version(service: &str) -> Option<&'static str> {
    expected_capability_version_for_service(service)
}

fn map_health_validation_error(error: ServiceHealthValidationError) -> ServiceManagerError {
    match error {
        ServiceHealthValidationError::ServiceIdentityMismatch { expected, actual } => {
            ServiceManagerError::ServiceIdentityMismatch { expected, actual }
        }
        ServiceHealthValidationError::Unhealthy { service, status } => {
            ServiceManagerError::Unhealthy { service, status }
        }
        ServiceHealthValidationError::ProtocolMismatch {
            service,
            expected,
            actual,
        } => ServiceManagerError::ProtocolMismatch {
            service,
            expected,
            actual,
        },
        ServiceHealthValidationError::CapabilityMismatch {
            service,
            expected,
            actual,
        } => ServiceManagerError::CapabilityMismatch {
            service,
            expected,
            actual,
        },
    }
}
