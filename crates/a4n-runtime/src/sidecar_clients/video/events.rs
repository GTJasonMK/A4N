use std::sync::Arc;

use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult};
use tokio::sync::mpsc::UnboundedSender;

use super::super::{
    GenerationEvent, parse_generation_event, service_error_string,
    service_request_with_context_provider_secret_policy,
    sidecar_context_for_workflow_operation_with_root,
};
use super::contracts::{VideoGenerationOutput, video_generation_result, video_provider_payload};

pub(in crate::sidecar_clients::video) async fn generate_video_via_sidecar(
    client: Arc<dyn a4n_capabilities::VideoClient>,
    config: &VideoProviderConfig,
    request: VideoGenerationRequest,
    provider_idempotency_key_sha256: Option<String>,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    progress_tx: Option<UnboundedSender<String>>,
) -> Result<VideoGenerationResult, String> {
    let payload =
        video_provider_payload(config, request, provider_idempotency_key_sha256, project_id)?;
    let request = service_request_with_context_provider_secret_policy(
        sidecar_context_for_workflow_operation_with_root(
            project_id,
            root_project_id,
            workflow_run_id,
            "video.generate",
        )?,
        a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
        payload,
    )?;
    let (event_tx, mut event_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
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
                match parse_generation_event::<VideoGenerationOutput>(event, "video")? {
                    GenerationEvent::Progress(message) => {
                        if let Some(progress_tx) = progress_tx.as_ref() {
                            progress_tx
                                .send(message)
                                .map_err(|_| "video sidecar progress 接收端已关闭".to_string())?;
                        }
                    }
                    GenerationEvent::Completed(output) => break output,
                }
            }
        }
    };
    video_generation_result(output)
}
