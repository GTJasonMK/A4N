use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse, sanitize_service_response,
};
use serde::{Serialize, de::DeserializeOwned};

use super::HttpSidecarTransport;
use super::errors::{timeout_error, transport_error};
use crate::contracts::{CapabilityResult, JsonRequest, JsonResponse};

impl HttpSidecarTransport {
    pub(crate) async fn post_typed<I, O>(
        &self,
        path: &str,
        request: ServiceRequest<I>,
    ) -> CapabilityResult<ServiceResponse<O>>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        from_json_response(self.post_json(path, into_json_request(request)?).await?)
    }

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
        sanitize_service_response(&mut response).map_err(|error| {
            ServiceError::non_retryable(
                ServiceErrorCode::Internal,
                format!("sidecar response artifact ref 非法: {error}"),
            )
        })?;
        Ok(response)
    }
}

pub(crate) fn into_json_request<T>(request: ServiceRequest<T>) -> CapabilityResult<JsonRequest>
where
    T: Serialize,
{
    let (context, capability_version, payload) = request.into_context_and_payload();
    let payload = serde_json::to_value(payload).map_err(|error| {
        ServiceError::non_retryable(
            ServiceErrorCode::ConfigurationError,
            format!("typed sidecar request payload serialization failed: {error}"),
        )
    })?;
    context.with_payload(capability_version, payload)
}

pub(crate) fn from_json_response<T>(response: JsonResponse) -> CapabilityResult<ServiceResponse<T>>
where
    T: DeserializeOwned,
{
    let ServiceResponse {
        status,
        trace_id,
        usage,
        artifact_refs,
        payload,
        error,
    } = response;
    let payload = payload
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| {
            ServiceError::non_retryable(
                ServiceErrorCode::Internal,
                format!("typed sidecar response payload deserialization failed: {error}"),
            )
        })?;
    Ok(ServiceResponse {
        status,
        trace_id,
        usage,
        artifact_refs,
        payload,
        error,
    })
}
