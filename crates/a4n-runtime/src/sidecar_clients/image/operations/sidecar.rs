use std::sync::Arc;

use a4n_capabilities::{A4nCapabilities, ImageClient};
use a4n_core::models::ai::image_gen_config;
use a4n_image::{ImageGenerationRequest, ImageGenerationResult};
use base64::Engine as _;
use serde_json::Value;

use super::super::super::{
    SidecarRequestContext, missing_sidecar_capability_message, response_payload,
    service_error_string, service_request_with_context_provider_secret_policy,
    sidecar_context_for_workflow_operation_with_root,
};
use super::super::{
    contracts::{ImageProviderProbeOutput, image_provider_payload},
    events::handle_image_generation_event,
};

pub(crate) async fn probe_image_config(
    capabilities: &A4nCapabilities,
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
) -> Result<(), String> {
    let client = capabilities
        .image
        .clone()
        .ok_or_else(|| missing_sidecar_capability_message("image"))?;
    let payload = image_provider_payload(config, request)?;
    let response = client
        .probe(service_request_with_context_provider_secret_policy(
            SidecarRequestContext::current_global(),
            "image",
            a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    let output: ImageProviderProbeOutput = response_payload(response, "image")?;
    if !output.ok {
        return Err("image sidecar probe 返回 ok=false".to_string());
    }
    if output.request_preview.is_null() {
        return Err("image sidecar probe 返回空 request_preview".to_string());
    }
    Ok(())
}

pub(crate) async fn preview_image_provider_request(
    capabilities: &A4nCapabilities,
    config: &image_gen_config::Model,
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
            "image",
            a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    let preview: Value = response_payload(response, "image")?;
    if preview.is_null() {
        return Err("image sidecar request preview 为空".to_string());
    }
    Ok(preview)
}

pub(super) async fn generate_image_via_sidecar(
    client: Arc<dyn ImageClient>,
    config: &image_gen_config::Model,
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
        "image",
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
        payload,
    )?;
    let (event_tx, mut event_rx) =
        tokio::sync::mpsc::channel(a4n_capabilities::EVENT_QUEUE_CAPACITY);
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
                if let Some(output) = handle_image_generation_event(event, progress)? {
                    break output;
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
