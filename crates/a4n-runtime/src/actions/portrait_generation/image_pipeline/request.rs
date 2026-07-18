use std::collections::HashSet;
use std::path::Path;

use base64::Engine as _;
use tokio_util::sync::CancellationToken;

use a4n_image::{ImageGenerationRequest, ImageReference};
use a4n_orchestrator::config::{
    configured_image_provider, image_backend_supports_reference_images,
};
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};

use crate::AppState;
use crate::actions::image_generation::resolve_image_config;
use crate::file_utils::canonicalize_data_file_path;

use super::contracts::{GeneratedPortraitImage, PortraitImageRequestContext};

pub(in crate::actions::portrait_generation) async fn build_portrait_image_request(
    state: &AppState,
    project_id: i64,
    prompt: &str,
    reference_image_ids: &[i64],
) -> Result<PortraitImageRequestContext, String> {
    let config = resolve_image_config(state, None).await?;
    let reference_images =
        collect_portrait_reference_images(state, project_id, reference_image_ids).await?;
    if !reference_images.is_empty() && !image_backend_supports_reference_images(&config.backend) {
        return Err(format!(
            "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除参考图",
            config.backend
        ));
    }
    Ok(PortraitImageRequestContext {
        request: ImageGenerationRequest {
            prompt: prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images,
        },
        config,
    })
}

async fn collect_portrait_reference_images(
    state: &AppState,
    project_id: i64,
    image_ids: &[i64],
) -> Result<Vec<ImageReference>, String> {
    let image_ids = validate_reference_image_ids(image_ids)?;
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    let images = state
        .orchestrator
        .portrait_reference_images_by_ids(project_id, &image_ids)
        .await
        .map_err(|err| err.to_string())?;
    let mut out = Vec::with_capacity(images.len());
    for (index, image) in images.iter().enumerate() {
        out.push(read_portrait_reference_image(
            state,
            &format!("参考图 {}", index + 1),
            &image.file_path,
        )?);
    }
    Ok(out)
}

pub(in crate::actions::portrait_generation) fn validate_reference_image_ids(
    image_ids: &[i64],
) -> Result<Vec<i64>, String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(image_ids.len());
    for image_id in image_ids.iter().copied() {
        if image_id <= 0 {
            return Err(format!("参考图 ID 必须为正整数：{image_id}"));
        }
        if !seen.insert(image_id) {
            return Err(format!("参考图包含重复 ID：{image_id}"));
        }
        normalized.push(image_id);
    }
    Ok(normalized)
}

fn read_portrait_reference_image(
    state: &AppState,
    label: &str,
    path: &str,
) -> Result<ImageReference, String> {
    let path = canonicalize_data_file_path(&state.config.data_dir, path, label)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("读取{label}失败：{e}"))?;
    let mime = portrait_reference_image_mime(&path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(ImageReference {
        label: label.to_string(),
        data_uri: format!("data:{mime};base64,{encoded}"),
    })
}

fn portrait_reference_image_mime(path: &Path) -> Result<&'static str, String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        other => Err(format!("参考图格式不支持: {other}，仅支持 png/jpg/webp")),
    }
}

pub(in crate::actions::portrait_generation) async fn run_portrait_image_generation(
    state: &AppState,
    project_id: i64,
    prompt: &str,
    reference_image_ids: &[i64],
    workflow_run_id: Option<i64>,
    cancel_token: CancellationToken,
) -> Result<GeneratedPortraitImage, String> {
    let context =
        build_portrait_image_request(state, project_id, prompt, reference_image_ids).await?;
    let provider_backend = context.config.backend.clone();
    let request_json = serde_json::to_value(&context.request)
        .map_err(|err| format!("人物立绘图片请求序列化失败: {err}"))?;
    let provider = configured_image_provider(&context.config).map_err(|e| e.to_string())?;
    let result = crate::actions::image_generation::run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::ImageGeneration,
        TaskPriority::Medium,
        Some(project_id),
        workflow_run_id,
        cancel_token,
        || async move {
            provider
                .generate(context.request)
                .await
                .map_err(|e| e.to_string())
        },
    )
    .await?;
    Ok(GeneratedPortraitImage {
        bytes: result.bytes,
        format: result.format,
        model: result.model,
        provider: provider_backend,
        request_json,
        width: context.config.default_width,
        height: context.config.default_height,
    })
}
