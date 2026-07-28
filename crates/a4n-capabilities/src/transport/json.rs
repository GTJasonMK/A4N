use a4n_service_protocol::{ServiceError, ServiceErrorCode, sanitize_service_response};

use super::HttpSidecarTransport;
use super::errors::{timeout_error, transport_error};
use crate::contracts::{CapabilityResult, JsonRequest, JsonResponse};

impl HttpSidecarTransport {
    pub(crate) async fn post_json(
        &self,
        path: &str,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse> {
        let client = self.request_client()?;
        let mut response = tokio::time::timeout(self.request_timeout, async {
            self.authorize_request(client.post(self.url(path)), &request)
                .json(&request)
                .send()
                .await
                .map_err(transport_error)?
                .error_for_status()
                .map_err(transport_error)?
                .json::<JsonResponse>()
                .await
                .map_err(transport_error)
        })
        .await
        .map_err(|_| timeout_error("request"))??;
        sanitize_service_response(&mut response).map_err(|error| ServiceError {
            code: ServiceErrorCode::Internal,
            message: format!("sidecar response artifact ref 非法: {error}"),
            retryable: Some(false),
        })?;
        Ok(response)
    }
}
