use a4n_core::error::AppError;
use a4n_service_protocol::{ServiceErrorCode, ServiceTaskEvent, ServiceTaskEventKind};
use serde::Deserialize;
use serde_json::Value;

use crate::sidecar_clients::{service_app_error, service_error_app_error};

pub(super) fn handle_rag_ingest_event(
    event: ServiceTaskEvent,
) -> Result<Option<WrittenOutput>, AppError> {
    match event.event_kind {
        ServiceTaskEventKind::Progress | ServiceTaskEventKind::StructuredProgress => {
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
        ServiceTaskEventKind::Completed => {
            let payload = event.payload.ok_or_else(|| {
                service_app_error("rag sidecar ingest completed 事件缺少 payload")
            })?;
            serde_json::from_value(payload).map(Some).map_err(|error| {
                service_app_error(format!(
                    "rag sidecar ingest completed payload 解析失败: {error}"
                ))
            })
        }
        ServiceTaskEventKind::Failed => {
            let error = event
                .error
                .ok_or_else(|| service_app_error("rag sidecar ingest failed 事件缺少 error"))?;
            Err(rag_service_error_app_error(error))
        }
        ServiceTaskEventKind::CancelAck => Err(AppError::Cancelled),
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
