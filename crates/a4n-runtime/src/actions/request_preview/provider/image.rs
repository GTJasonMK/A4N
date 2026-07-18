use a4n_image::ImageGenerationRequest;
#[cfg(feature = "embedded-media-providers")]
use a4n_image_providers::{
    ChatCompletionsImageProvider, CliProxyImageProvider, ComfyUiImageProvider,
    OpenAiCompatibleImageEditProvider, OpenAiCompatibleImageProvider,
    StableDiffusionWebUiImageProvider,
};
use serde_json::Value;

use super::super::contracts::ModelRequestPreviewStep;
use super::generic::{blocked_step, provider_request_step};

pub(crate) fn image_provider_request_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    state: impl Into<String>,
    note: impl Into<Option<String>>,
    backend: &str,
    request: ImageGenerationRequest,
    provider_options_json: Option<&Value>,
) -> Result<ModelRequestPreviewStep, String> {
    let key = key.into();
    let title = title.into();
    let prompt_name = prompt_name.into();
    let state = state.into();
    let note = note.into();
    match image_provider_preview_payload(backend, &request, provider_options_json) {
        Ok((kind, provider_note, provider_request)) => provider_request_step(
            key,
            title,
            kind,
            prompt_name,
            state,
            join_preview_notes(note, Some(provider_note)),
            &provider_request,
        ),
        Err(err) => Ok(blocked_step(
            key,
            title,
            "image_generation",
            prompt_name,
            join_preview_notes(note, Some(err))
                .unwrap_or_else(|| "图片 provider 请求不可用".into()),
        )),
    }
}

#[cfg(feature = "embedded-media-providers")]
fn image_provider_preview_payload(
    backend: &str,
    request: &ImageGenerationRequest,
    provider_options_json: Option<&Value>,
) -> Result<(&'static str, String, Value), String> {
    match backend {
        "openai-compatible-image" => Ok((
            "image_generation_openai_compatible",
            "OpenAI 兼容 `/images/generations` JSON 请求；该后端不支持参考图输入。".into(),
            OpenAiCompatibleImageProvider::request_body_preview(request)
                .map_err(|err| err.to_string())?,
        )),
        "openai-compatible-image-edit" => {
            let note = if request.reference_images.is_empty() {
                "OpenAI 兼容图片编辑后端在无参考图时走 `/images/generations` JSON 请求。"
            } else {
                "OpenAI 兼容 `/images/edits` multipart 请求；预览省略参考图媒体数据。"
            };
            Ok((
                "image_generation_openai_compatible_edit",
                note.into(),
                OpenAiCompatibleImageEditProvider::request_body_preview(request)
                    .map_err(|err| err.to_string())?,
            ))
        }
        "openai-chat-image" => Ok((
            "image_generation_chat_completions",
            "OpenAI Chat-Completions 图像请求；预览省略参考图媒体数据。".into(),
            ChatCompletionsImageProvider::request_body_preview(request),
        )),
        "cliproxy-image" => {
            let note = if request.reference_images.is_empty() {
                "CLIProxy `/images/generations` JSON 请求；强制 response_format=url。"
            } else {
                "CLIProxy `/images/edits` multipart 请求；强制 response_format=url，预览省略参考图媒体数据。"
            };
            Ok((
                "image_generation_cliproxy",
                note.into(),
                CliProxyImageProvider::request_body_preview_with_options(
                    request,
                    provider_options_json,
                )
                .map_err(|err| err.to_string())?,
            ))
        }
        "stable-diffusion" => Ok((
            "image_generation_sd_webui",
            "Stable Diffusion WebUI `/sdapi/v1/txt2img` JSON 请求。".into(),
            StableDiffusionWebUiImageProvider::request_body_preview(request)
                .map_err(|err| err.to_string())?,
        )),
        "comfyui" => Ok((
            "image_generation_comfyui",
            "ComfyUI `/prompt` workflow 请求；真实生成时 client_id 与 seed 会动态生成。".into(),
            ComfyUiImageProvider::request_body_preview(request).map_err(|err| err.to_string())?,
        )),
        other => Err(format!(
            "未知图片 backend={other}，无法展示真实 provider 请求体"
        )),
    }
}

#[cfg(not(feature = "embedded-media-providers"))]
fn image_provider_preview_payload(
    backend: &str,
    _request: &ImageGenerationRequest,
    _provider_options_json: Option<&Value>,
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
