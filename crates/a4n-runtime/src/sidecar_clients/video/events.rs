use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::ai::video_gen_config;
use a4n_service_protocol::{ServiceErrorCode, ServiceTaskEvent, ServiceTaskEventKind};
use a4n_video::{VideoGenerationRequest, VideoGenerationResult};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use super::super::{
    service_error_string, service_request_with_context_provider_secret_policy,
    sidecar_context_for_workflow_operation_with_root,
};
use super::contracts::{VideoGenerationOutput, video_generation_result, video_provider_payload};

pub(in crate::sidecar_clients::video) async fn generate_video_via_sidecar(
    client: Arc<dyn a4n_capabilities::VideoClient>,
    config: &video_gen_config::Model,
    request: VideoGenerationRequest,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    progress_tx: Option<UnboundedSender<String>>,
) -> Result<VideoGenerationResult, String> {
    let payload = video_provider_payload(config, request, project_id)?;
    let request = service_request_with_context_provider_secret_policy(
        sidecar_context_for_workflow_operation_with_root(
            project_id,
            root_project_id,
            workflow_run_id,
            "video.generate",
        )?,
        "video",
        a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
        payload,
    )?;
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut request_future = Box::pin(client.generate_events(request, event_tx));
    let mut transport_complete = false;
    let output = loop {
        tokio::select! {
            result = &mut request_future, if !transport_complete => {
                result.map_err(service_error_string)?;
                transport_complete = true;
            }
            event = event_rx.recv() => {
                let event = event.ok_or_else(|| {
                    "video sidecar 生成事件流结束但缺少终止事件".to_string()
                })?;
                if let Some(output) = handle_video_generation_event(event, progress_tx.as_ref())? {
                    break output;
                }
            }
        }
    };
    video_generation_result(output)
}

pub(in crate::sidecar_clients::video) fn handle_video_generation_event(
    event: ServiceTaskEvent,
    progress_tx: Option<&UnboundedSender<String>>,
) -> Result<Option<VideoGenerationOutput>, String> {
    match event.event_kind {
        ServiceTaskEventKind::Progress | ServiceTaskEventKind::StructuredProgress => {
            let payload = event
                .payload
                .ok_or_else(|| "video sidecar progress 事件缺少 payload".to_string())?;
            let message = payload
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .ok_or_else(|| "video sidecar progress 事件缺少 message".to_string())?;
            if let Some(progress_tx) = progress_tx {
                progress_tx
                    .send(message.to_string())
                    .map_err(|_| "video sidecar progress 接收端已关闭".to_string())?;
            }
            Ok(None)
        }
        ServiceTaskEventKind::Completed => {
            let payload = event
                .payload
                .ok_or_else(|| "video sidecar completed 事件缺少 payload".to_string())?;
            serde_json::from_value(payload)
                .map(Some)
                .map_err(|error| format!("video sidecar completed payload 解析失败: {error}"))
        }
        ServiceTaskEventKind::Failed => {
            let error = event
                .error
                .ok_or_else(|| "video sidecar failed 事件缺少 error".to_string())?;
            if error.code == ServiceErrorCode::Cancelled {
                Err(AppError::Cancelled.to_string())
            } else {
                Err(service_error_string(error))
            }
        }
        ServiceTaskEventKind::CancelAck => Err(AppError::Cancelled.to_string()),
        other => Err(format!("video sidecar 生成事件类型不符合合同: {other:?}")),
    }
}
