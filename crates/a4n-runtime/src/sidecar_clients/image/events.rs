use a4n_core::error::AppError;
use a4n_service_protocol::{ServiceErrorCode, ServiceTaskEvent, ServiceTaskEventKind};
use serde_json::Value;

use super::contracts::ImageGenerationOutput;
use crate::sidecar_clients::service_error_string;

pub(super) fn handle_image_generation_event(
    event: ServiceTaskEvent,
    progress: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<Option<ImageGenerationOutput>, String> {
    match event.event_kind {
        ServiceTaskEventKind::Progress | ServiceTaskEventKind::StructuredProgress => {
            let payload = event
                .payload
                .ok_or_else(|| "image sidecar progress 事件缺少 payload".to_string())?;
            let message = payload
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .ok_or_else(|| "image sidecar progress 事件缺少 message".to_string())?;
            if let Some(progress) = progress {
                progress(message);
            }
            Ok(None)
        }
        ServiceTaskEventKind::Completed => {
            let payload = event
                .payload
                .ok_or_else(|| "image sidecar completed 事件缺少 payload".to_string())?;
            serde_json::from_value(payload)
                .map(Some)
                .map_err(|error| format!("image sidecar completed payload 解析失败: {error}"))
        }
        ServiceTaskEventKind::Failed => {
            let error = event
                .error
                .ok_or_else(|| "image sidecar failed 事件缺少 error".to_string())?;
            if error.code == ServiceErrorCode::Cancelled {
                Err(AppError::Cancelled.to_string())
            } else {
                Err(service_error_string(error))
            }
        }
        ServiceTaskEventKind::CancelAck => Err(AppError::Cancelled.to_string()),
        other => Err(format!("image sidecar 生成事件类型不符合合同: {other:?}")),
    }
}
