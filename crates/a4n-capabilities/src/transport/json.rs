use a4n_service_protocol::{ServiceError, ServiceErrorCode, sanitize_service_response};

use super::HttpSidecarTransport;
use super::errors::transport_error;
use crate::contracts::{CapabilityResult, JsonRequest, JsonResponse};

impl HttpSidecarTransport {
    pub(crate) async fn post_json(
        &self,
        path: &str,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse> {
        let mut response = self
            .client
            .post(self.url(path))
            .json(&request)
            .send()
            .await
            .map_err(transport_error)?
            .error_for_status()
            .map_err(transport_error)?
            .json::<JsonResponse>()
            .await
            .map_err(transport_error)?;
        sanitize_service_response(&mut response).map_err(|error| ServiceError {
            code: ServiceErrorCode::Internal,
            message: format!("sidecar response artifact ref 非法: {error}"),
            retryable: Some(false),
        })?;
        Ok(response)
    }
}
