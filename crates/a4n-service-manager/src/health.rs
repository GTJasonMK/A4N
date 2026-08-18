use super::config::ManagedServiceConfig;
use super::config::ServiceMode;
use super::errors::ServiceManagerError;
use a4n_service_protocol::{
    SIDECAR_AUTH_HEADER, SIDECAR_HEALTH_PATH, ServiceHealth, ServiceHealthValidationError,
    expected_capability_version_for_service, validate_service_health_envelope,
};

pub(super) async fn http_health(
    client: &reqwest::Client,
    config: &ManagedServiceConfig,
    auth_token: Option<&str>,
) -> Result<ServiceHealth, ServiceManagerError> {
    let url = config.endpoint_url(SIDECAR_HEALTH_PATH)?;
    let mut request = client.get(&url);
    if let Some(token) = auth_token {
        request = request.header(SIDECAR_AUTH_HEADER, token);
    }
    let response = request
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
        config.name.as_str(),
        expected_capability_version_for_service(&config.name),
    )
    .map_err(map_health_validation_error)?;
    if matches!(config.mode, ServiceMode::Sidecar)
        && expected_capability_version_for_service(&config.name).is_some()
        && health.version.service_version != env!("CARGO_PKG_VERSION")
    {
        return Err(ServiceManagerError::ServiceBuildMismatch {
            service: config.name.clone(),
            expected: env!("CARGO_PKG_VERSION").to_string(),
            actual: health.version.service_version.clone(),
        });
    }
    Ok(())
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
