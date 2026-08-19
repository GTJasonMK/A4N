use a4n_capabilities::{JsonResponse, sidecar_response_payload};
use a4n_core::error::AppError;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceErrorCode, ServiceResponse,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(super) enum GenerationEvent<T> {
    Progress(String),
    Completed(T),
}

pub(super) fn parse_generation_event<T: DeserializeOwned>(
    event: CapabilityTaskEvent,
    service: &str,
) -> Result<GenerationEvent<T>, String> {
    match event.event_kind {
        CapabilityTaskEventKind::Progress => {
            let payload = event
                .payload
                .ok_or_else(|| format!("{service} sidecar progress 事件缺少 payload"))?;
            let message = payload
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .ok_or_else(|| format!("{service} sidecar progress 事件缺少 message"))?;
            Ok(GenerationEvent::Progress(message.to_string()))
        }
        CapabilityTaskEventKind::Completed => {
            let payload = event
                .payload
                .ok_or_else(|| format!("{service} sidecar completed 事件缺少 payload"))?;
            serde_json::from_value(payload)
                .map(GenerationEvent::Completed)
                .map_err(|error| format!("{service} sidecar completed payload 解析失败: {error}"))
        }
        CapabilityTaskEventKind::Failed => {
            let error = event
                .error
                .ok_or_else(|| format!("{service} sidecar failed 事件缺少 error"))?;
            Err(service_error_string(error))
        }
        CapabilityTaskEventKind::Chunk => Err(format!(
            "{service} sidecar generation 事件类型不符合合同: chunk"
        )),
    }
}

pub(crate) fn response_payload<T: DeserializeOwned>(
    response: JsonResponse,
    service: &str,
) -> Result<T, String> {
    serde_json::from_value(typed_response_payload(response, service)?)
        .map_err(|err| format!("{service} sidecar 响应 payload 解析失败: {err}"))
}

pub(super) fn typed_response_payload<T>(
    response: ServiceResponse<T>,
    service: &str,
) -> Result<T, String> {
    sidecar_response_payload(response, service).map_err(service_error_string)
}

pub(super) fn service_error_string(error: a4n_service_protocol::ServiceError) -> String {
    if error.code == ServiceErrorCode::Cancelled {
        AppError::Cancelled.to_string()
    } else {
        error.message
    }
}

pub(super) fn service_app_error(message: impl Into<String>) -> AppError {
    AppError::ConfigError(message.into())
}

pub(super) fn service_error_app_error(error: a4n_service_protocol::ServiceError) -> AppError {
    if error.code == ServiceErrorCode::Cancelled {
        AppError::Cancelled
    } else {
        AppError::ConfigError(error.message)
    }
}
