#[cfg(feature = "embedded-media-providers")]
use a4n_domain::image::{
    CLIPROXY_IMAGE_BACKEND, COMFYUI_IMAGE_BACKEND, OPENAI_CHAT_IMAGE_BACKEND,
    OPENAI_COMPATIBLE_IMAGE_BACKEND, OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND,
    STABLE_DIFFUSION_IMAGE_BACKEND,
};
use a4n_image_contract::ImageGenerationRequest;
#[cfg(feature = "embedded-media-providers")]
use a4n_image_providers::image_provider_request_body_preview;
use serde_json::Value;

use super::super::contracts::ModelRequestPreviewStep;
use super::generic::provider_request_step;

pub(crate) fn image_provider_request_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    state: impl Into<String>,
    note: impl Into<Option<String>>,
    backend: &str,
    request: ImageGenerationRequest,
) -> Result<ModelRequestPreviewStep, String> {
    let key = key.into();
    let title = title.into();
    let prompt_name = prompt_name.into();
    let state = state.into();
    let note = note.into();
    let (kind, provider_note, provider_request) =
        image_provider_preview_payload(backend, &request)?;
    provider_request_step(
        key,
        title,
        kind,
        prompt_name,
        state,
        join_preview_notes(note, Some(provider_note)),
        &provider_request,
    )
}

#[cfg(feature = "embedded-media-providers")]
fn image_provider_preview_payload(
    backend: &str,
    request: &ImageGenerationRequest,
) -> Result<(&'static str, String, Value), String> {
    let (kind, note) = match backend {
        OPENAI_COMPATIBLE_IMAGE_BACKEND => (
            "image_generation_openai_compatible",
            "OpenAI 兼容 `/images/generations` JSON 请求；该后端不支持参考图输入。".into(),
        ),
        OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND => {
            let note = if request.reference_images.is_empty() {
                "OpenAI 兼容图片编辑后端在无参考图时走 `/images/generations` JSON 请求。"
            } else {
                "OpenAI 兼容 `/images/edits` multipart 请求；预览省略参考图媒体数据。"
            };
            ("image_generation_openai_compatible_edit", note.into())
        }
        OPENAI_CHAT_IMAGE_BACKEND => (
            "image_generation_chat_completions",
            "OpenAI Chat-Completions 图像请求；预览省略参考图媒体数据。".into(),
        ),
        CLIPROXY_IMAGE_BACKEND => {
            let note = if request.reference_images.is_empty() {
                "CLIProxy `/images/generations` JSON 请求；强制 response_format=url。"
            } else {
                "CLIProxy `/images/edits` multipart 请求；强制 response_format=url，预览省略参考图媒体数据。"
            };
            ("image_generation_cliproxy", note.into())
        }
        STABLE_DIFFUSION_IMAGE_BACKEND => (
            "image_generation_sd_webui",
            "Stable Diffusion WebUI `/sdapi/v1/txt2img` JSON 请求。".into(),
        ),
        COMFYUI_IMAGE_BACKEND => (
            "image_generation_comfyui",
            "ComfyUI `/prompt` workflow 请求；真实生成时 client_id 与 seed 会动态生成。".into(),
        ),
        other => {
            return Err(format!(
                "未知图片 backend={other}，无法展示真实 provider 请求体"
            ));
        }
    };
    let provider_request =
        image_provider_request_body_preview(backend, request).map_err(|err| err.to_string())?;
    Ok((kind, note, provider_request))
}

#[cfg(not(feature = "embedded-media-providers"))]
fn image_provider_preview_payload(
    backend: &str,
    _request: &ImageGenerationRequest,
) -> Result<(&'static str, String, Value), String> {
    Err(format!(
        "图片 provider 请求预览未启用：backend={backend}；请使用 image sidecar，或用 embedded-media-providers feature 构建"
    ))
}

fn join_preview_notes(first: Option<String>, second: Option<String>) -> Option<String> {
    let notes = [first, second]
        .into_iter()
        .flatten()
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty())
        .collect::<Vec<_>>();
    if notes.is_empty() {
        None
    } else {
        Some(notes.join("；"))
    }
}
