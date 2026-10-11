use std::sync::Arc;

use a4n_image_client::ImageClient;
use a4n_image_contract::ImageGenerationRequest;
use a4n_image_provider_contract::ImageGenerationResult;
use a4n_provider_contract::ImageProviderConfig;
use base64::Engine as _;
use serde_json::Value;

use super::super::super::{
    GenerationStreamError, SidecarRequestContext, SidecarWorkflowScope, collect_generation_output,
    missing_sidecar_capability_message, response_payload, service_error_string,
    sidecar_context_for_workflow_operation_with_root,
    typed_service_request_with_context_provider_secret_policy,
};
use super::super::contracts::{ImageGenerationOutput, image_provider_payload};

pub(crate) async fn preview_image_provider_request(
    client: Option<Arc<dyn ImageClient>>,
    config: &ImageProviderConfig,
    request: ImageGenerationRequest,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<Value, String> {
    let client = client.ok_or_else(|| missing_sidecar_capability_message("image"))?;
    let payload = image_provider_payload(config, request)?;
    let response = client
        .preview_request(typed_service_request_with_context_provider_secret_policy(
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
    scope: SidecarWorkflowScope,
    progress: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<ImageGenerationResult, String> {
    let payload = image_provider_payload(config, request)?;
    let request = typed_service_request_with_context_provider_secret_policy(
        sidecar_context_for_workflow_operation_with_root(scope, "image.generate")?,
        a4n_service_protocol::IMAGE_PROVIDER_CAPABILITY_VERSION,
        payload,
    )?;
    let (event_tx, event_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
    let output = collect_generation_output::<ImageGenerationOutput, _, _>(
        client.generate_events(request, event_tx),
        event_rx,
        "image",
        |message| {
            if let Some(progress) = progress {
                progress(&message);
            }
            Ok::<(), String>(())
        },
    )
    .await
    .map_err(|error| match error {
        GenerationStreamError::Transport(error) => service_error_string(error),
        GenerationStreamError::Event(error) | GenerationStreamError::Progress(error) => error,
    })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(output.bytes_base64)
        .map_err(|err| format!("image sidecar 返回的 bytes_base64 无效: {err}"))?;
    Ok(ImageGenerationResult {
        bytes,
        format: output.format,
        model: output.model,
    })
}
