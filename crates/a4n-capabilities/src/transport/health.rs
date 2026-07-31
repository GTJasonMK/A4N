use a4n_service_protocol::{
    PROTOCOL_VERSION, SIDECAR_HOST_PROTOCOL_VERSION_HEADER, ServiceError, ServiceErrorCode,
    ServiceHealth, validate_service_health_envelope,
};

use super::HttpSidecarTransport;
use super::errors::{timeout_error, transport_error};
use crate::contracts::CapabilityResult;

impl HttpSidecarTransport {
    pub(crate) async fn health(&self) -> CapabilityResult<ServiceHealth> {
        validate_health_expectation(self.expected_service_name, self.expected_capability_version)?;
        let client = self.request_client()?;
        let health = tokio::time::timeout(self.health_timeout, async {
            self.authorize(client.get(self.url("/healthz")).header(
                SIDECAR_HOST_PROTOCOL_VERSION_HEADER,
                PROTOCOL_VERSION.to_string(),
            ))
            .send()
            .await
            .map_err(transport_error)?
            .error_for_status()
            .map_err(transport_error)?
            .json::<ServiceHealth>()
            .await
            .map_err(transport_error)
        })
        .await
        .map_err(|_| timeout_error("health request"))??;
        validate_service_health(
            &health,
            self.expected_service_name,
            self.expected_capability_version,
        )?;
        Ok(health)
    }
}

pub(crate) fn validate_health_expectation(
    expected_service_name: Option<&str>,
    expected_capability_version: Option<&str>,
) -> CapabilityResult<()> {
    if let Some(service) = expected_service_name
        && expected_capability_version.is_none()
    {
        return Err(ServiceError::non_retryable(
            ServiceErrorCode::ConfigurationError,
            format!("typed HTTP client service {service} 未注册 capability version"),
        ));
    }
    Ok(())
}

pub(crate) fn validate_service_health(
    health: &ServiceHealth,
    expected_service_name: Option<&str>,
    expected_capability_version: Option<&str>,
) -> CapabilityResult<()> {
    validate_service_health_envelope(health, expected_service_name, expected_capability_version)
        .map_err(|error| error.into_service_error())
}
