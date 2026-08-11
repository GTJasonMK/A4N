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

pub(super) fn typed_response_payload<T>(
    mut response: ServiceResponse<T>,
    service: &str,
) -> Result<T, String> {
    sanitize_service_response(&mut response)
        .map_err(|err| format!("{service} sidecar 返回非法 response: {err}"))?;
    match response.status {
        ServiceStatus::Ok => response
            .payload
            .ok_or_else(|| format!("{service} sidecar 返回成功但缺少 payload")),
        ServiceStatus::Error => Err(response
            .error
            .map(|error| error.message)
            .unwrap_or_else(|| format!("{service} sidecar 返回未知错误"))),
    }
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

pub(super) fn service_response_unit(
    mut response: ServiceResponse<Value>,
    service: &str,
) -> Result<(), String> {
    sanitize_service_response(&mut response)
        .map_err(|err| format!("{service} sidecar 返回非法 response: {err}"))?;
    match response.status {
        ServiceStatus::Ok if response.payload.is_none() => Ok(()),
        ServiceStatus::Ok => Err(format!("{service} sidecar unit 响应不应包含 payload")),
        ServiceStatus::Error => Err(response
            .error
            .map(|error| error.message)
            .unwrap_or_else(|| format!("{service} sidecar 返回未知错误"))),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_response(payload: Option<Value>) -> ServiceResponse<Value> {
        ServiceResponse {
            status: ServiceStatus::Ok,
            trace_id: Some("trace-1".to_string()),
            usage: None,
            artifact_refs: Vec::new(),
            payload,
            error: None,
        }
    }

    #[test]
    fn unit_response_rejects_legacy_payloads() {
        service_response_unit(ok_response(None), "embedding").expect("unit response");
        assert!(
            service_response_unit(
                ok_response(Some(serde_json::json!({ "deleted": true }))),
                "embedding",
            )
            .is_err()
        );
    }
}
