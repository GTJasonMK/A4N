use a4n_capabilities::JsonResponse;
use a4n_core::error::AppError;
use a4n_service_protocol::{ServiceResponse, ServiceStatus, sanitize_service_response};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(crate) fn response_payload<T: DeserializeOwned>(
    response: JsonResponse,
    service: &str,
) -> Result<T, String> {
    service_response_payload(response, service)
}

pub(super) fn service_response_payload<T: DeserializeOwned>(
    mut response: ServiceResponse<Value>,
    service: &str,
) -> Result<T, String> {
    sanitize_service_response(&mut response)
        .map_err(|err| format!("{service} sidecar 返回非法 response: {err}"))?;
    match response.status {
        ServiceStatus::Ok => {
            let payload = response
                .payload
                .ok_or_else(|| format!("{service} sidecar 返回成功但缺少 payload"))?;
            serde_json::from_value(payload)
                .map_err(|err| format!("{service} sidecar 响应 payload 解析失败: {err}"))
        }
        ServiceStatus::Error => {
            let message = response
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| format!("{service} sidecar 返回未知错误"));
            Err(message)
        }
    }
}

pub(super) fn service_error_string(error: a4n_service_protocol::ServiceError) -> String {
    error.message
}

pub(super) fn service_app_error(message: impl Into<String>) -> AppError {
    AppError::ConfigError(message.into())
}

pub(super) fn service_error_app_error(error: a4n_service_protocol::ServiceError) -> AppError {
    AppError::ConfigError(error.message)
}
