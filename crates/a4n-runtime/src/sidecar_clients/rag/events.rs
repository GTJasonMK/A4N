use a4n_core::error::AppError;
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind, ServiceErrorCode};
use serde::Deserialize;
use serde_json::Value;

use crate::sidecar_clients::{service_app_error, service_error_app_error};

pub(super) fn handle_rag_ingest_event(
    event: CapabilityTaskEvent,
) -> Result<Option<WrittenOutput>, AppError> {
    match event.event_kind {
        CapabilityTaskEventKind::Progress | CapabilityTaskEventKind::StructuredProgress => {
            let payload = event
                .payload
                .ok_or_else(|| service_app_error("rag sidecar ingest progress 事件缺少 payload"))?;
            payload
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .ok_or_else(|| service_app_error("rag sidecar ingest progress 事件缺少 message"))?;
            Ok(None)
        }
        CapabilityTaskEventKind::Completed => {
            let payload = event.payload.ok_or_else(|| {
                service_app_error("rag sidecar ingest completed 事件缺少 payload")
            })?;
            serde_json::from_value(payload).map(Some).map_err(|error| {
                service_app_error(format!(
                    "rag sidecar ingest completed payload 解析失败: {error}"
                ))
            })
        }
        CapabilityTaskEventKind::Failed => {
            let error = event
                .error
                .ok_or_else(|| service_app_error("rag sidecar ingest failed 事件缺少 error"))?;
            Err(rag_service_error_app_error(error))
        }
        CapabilityTaskEventKind::CancelAck => Err(AppError::Cancelled),
        other => Err(service_app_error(format!(
            "rag sidecar ingest 事件类型不符合合同: {other:?}"
        ))),
    }
}

pub(super) fn rag_service_error_app_error(error: a4n_service_protocol::ServiceError) -> AppError {
    if error.code == ServiceErrorCode::Cancelled {
        AppError::Cancelled
    } else {
        service_error_app_error(error)
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct WrittenOutput {
    pub(super) written: usize,
}
