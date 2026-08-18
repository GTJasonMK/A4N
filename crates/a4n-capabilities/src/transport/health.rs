use a4n_service_protocol::{
    SIDECAR_HEALTH_PATH, ServiceHealth, validate_service_health_envelope,
};

use super::HttpSidecarTransport;
use super::errors::{timeout_error, transport_error};
use crate::contracts::CapabilityResult;

impl HttpSidecarTransport {
    pub(crate) async fn health(&self) -> CapabilityResult<ServiceHealth> {
        let client = self.request_client()?;
        let health = tokio::time::timeout(self.health_timeout, async {
            self.authorize(client.get(self.url(SIDECAR_HEALTH_PATH)))
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

pub(crate) fn validate_service_health(
    health: &ServiceHealth,
    expected_service_name: &str,
    expected_capability_version: &str,
) -> CapabilityResult<()> {
    validate_service_health_envelope(
        health,
        expected_service_name,
        Some(expected_capability_version),
    )
    .map_err(|error| error.into_service_error())
}
