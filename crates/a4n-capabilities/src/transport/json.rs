use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse, ServiceStatus,
    sanitize_service_response, validate_service_request_context,
};
use reqwest::header::CONTENT_TYPE;
use serde::{Serialize, de::DeserializeOwned};

use super::HttpSidecarTransport;
use super::errors::{timeout_error, transport_error};
use crate::contracts::CapabilityResult;

pub fn sidecar_response_payload<T>(
    mut response: ServiceResponse<T>,
    service: &str,
) -> CapabilityResult<T> {
    sanitize_service_response(&mut response).map_err(|error| {
        invalid_sidecar_response(service, format!("返回非法 response: {error}"))
    })?;
    match (response.status, response.payload, response.error) {
        (ServiceStatus::Ok, Some(payload), None) => Ok(payload),
        (ServiceStatus::Ok, _, Some(_)) => Err(invalid_sidecar_response(
            service,
            "返回成功状态但同时携带 error",
        )),
        (ServiceStatus::Ok, None, None) => Err(invalid_sidecar_response(
            service,
            "返回成功但缺少 payload",
        )),
        (ServiceStatus::Error, Some(_), _) => Err(
            invalid_sidecar_response(service, "返回错误状态但同时携带 payload"),
        ),
        (ServiceStatus::Error, None, Some(error)) => Err(error),
        (ServiceStatus::Error, None, None) => Err(
            invalid_sidecar_response(service, "返回错误状态但缺少 error"),
        ),
    }
}

fn invalid_sidecar_response(service: &str, message: impl std::fmt::Display) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::Internal,
        format!("{service} sidecar {message}"),
    )
}

impl HttpSidecarTransport {
    pub(crate) async fn post<I, O>(
        &self,
        path: &str,
        request: ServiceRequest<I>,
    ) -> CapabilityResult<ServiceResponse<O>>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        let body = serialize_request(&request)?;
        let client = self.request_client()?;
        let mut response = tokio::time::timeout(self.request_timeout, async {
            self.authorize_request(client.post(self.url(path)), &request)?
                .header(CONTENT_TYPE, "application/json")
                .body(body)
                .send()
                .await
                .map_err(transport_error)?
                .error_for_status()
                .map_err(transport_error)?
                .json::<ServiceResponse<O>>()
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

pub(crate) fn serialize_request<T>(request: &ServiceRequest<T>) -> CapabilityResult<Vec<u8>>
where
    T: Serialize,
{
    validate_service_request_context(request)?;
    serde_json::to_vec(request).map_err(|error| {
        ServiceError::non_retryable(
            ServiceErrorCode::ConfigurationError,
            format!("sidecar request serialization failed: {error}"),
        )
    })
}
