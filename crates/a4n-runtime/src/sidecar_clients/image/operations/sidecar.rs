use std::sync::Arc;

use a4n_capabilities::{A4nCapabilities, ImageClient};
use a4n_image::{ImageGenerationRequest, ImageGenerationResult};
use a4n_provider_contract::ImageProviderConfig;
use base64::Engine as _;
use serde_json::Value;

use super::super::super::{
    GenerationEvent, SidecarRequestContext, missing_sidecar_capability_message,
    parse_generation_event, response_payload, service_error_string,
    service_request_with_context_provider_secret_policy,
    sidecar_context_for_workflow_operation_with_root,
};
use super::super::contracts::{ImageGenerationOutput, image_provider_payload};

pub(crate) async fn preview_image_provider_request(
    capabilities: &A4nCapabilities,
    config: &ImageProviderConfig,
    request: ImageGenerationRequest,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<Value, String> {
    let client = capabilities
        .image
        .clone()
        .ok_or_else(|| missing_sidecar_capability_message("image"))?;
    let payload = image_provider_payload(config, request)?;
    let response = client
        .preview_request(service_request_with_context_provider_secret_policy(
            SidecarRequestContext::current_global().with_project_scope(project_id, root_project_id),
            a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    response_payload(response, "image")
}

pub(super) async fn generate_image_via_sidecar(
    client: Arc<dyn ImageClient>,
    config: &ImageProviderConfig,
    request: ImageGenerationRequest,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    progress: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<ImageGenerationResult, String> {
    let payload = image_provider_payload(config, request)?;
    let request = service_request_with_context_provider_secret_policy(
        sidecar_context_for_workflow_operation_with_root(
            project_id,
            root_project_id,
            workflow_run_id,
            "image.generate",
        )?,
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
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
                    "image sidecar 生成事件流结束但缺少终止事件".to_string()
                })?;
                match parse_generation_event::<ImageGenerationOutput>(event, "image")? {
                    GenerationEvent::Progress(message) => {
                        if let Some(progress) = progress {
                            progress(&message);
                        }
                    }
                    GenerationEvent::Completed(output) => break output,
                }
            }
        }
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(output.bytes_base64)
        .map_err(|err| format!("image sidecar 返回的 bytes_base64 无效: {err}"))?;
    Ok(ImageGenerationResult {
        bytes,
        format: output.format,
        model: output.model,
    })
}
