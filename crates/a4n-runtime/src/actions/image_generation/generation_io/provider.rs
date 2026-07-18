use super::super::ImageGenerationProgress;
use super::queue::run_queued_request_with_cancel_and_workflow;
use crate::AppState;
use a4n_core::models::ai::image_gen_config;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_image::{ImageGenerationRequest, ImageGenerationResult};
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use serde_json::{Value, json};

pub(in crate::actions::image_generation) async fn run_image_generation_provider_request_with_workflow(
    state: &AppState,
    project_id: i64,
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
    workflow_run_id: Option<i64>,
    progress: &ImageGenerationProgress<'_>,
) -> Result<ImageGenerationResult, String> {
    let root_project_id = crate::actions::sidecar_root_project_id(state, project_id).await?;
    run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::ImageGeneration,
        TaskPriority::Medium,
        Some(project_id),
        workflow_run_id,
        progress.cancel_token(),
        || async move {
            let sidecar_progress = |message: &str| progress.progress(3, 4, message);
            crate::sidecar_clients::generate_image(
                state.runtime_profile,
                state.capabilities(),
                config,
                request,
                Some(project_id),
                root_project_id,
                workflow_run_id,
                Some(&sidecar_progress),
            )
            .await
        },
    )
    .await
}

pub(in crate::actions::image_generation) fn image_generation_request_trace_json(
    config: &image_gen_config::Model,
    request: &ImageGenerationRequest,
) -> Value {
    let reference_images = request
        .reference_images
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            json!({
                "index": index,
                "label": reference.label,
                "data_uri_sha256": source_content_sha256(&reference.data_uri),
                "data_uri_length": reference.data_uri.len(),
                "media_type": reference
                    .data_uri
                    .strip_prefix("data:")
                    .and_then(|value| value.split_once(';').map(|(media_type, _)| media_type)),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "provider": {
            "config_id": config.id,
            "config_name": config.name,
            "backend": config.backend,
            "base_url": config.base_url,
            "configured_model": config.model,
        },
        "request": {
            "prompt": request.prompt,
            "width": request.width,
            "height": request.height,
            "aspect_ratio": image_request_aspect_ratio(request.width, request.height),
            "model": request.model,
            "reference_image_count": request.reference_images.len(),
            "reference_images": reference_images,
        },
    })
}

fn image_request_aspect_ratio(width: i32, height: i32) -> String {
    let gcd = gcd_i32(width.abs().max(1), height.abs().max(1));
    format!("{}:{}", width.abs().max(1) / gcd, height.abs().max(1) / gcd)
}

fn gcd_i32(mut left: i32, mut right: i32) -> i32 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left.max(1)
}

#[cfg(test)]
pub(super) fn test_image_request_aspect_ratio(width: i32, height: i32) -> String {
    image_request_aspect_ratio(width, height)
}
