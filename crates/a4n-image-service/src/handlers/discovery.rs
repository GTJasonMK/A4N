use a4n_core::error::AppError;
use a4n_image_providers::{
    ChatCompletionsImageProvider, CliProxyImageProvider, ComfyUiImageProvider,
    OpenAiCompatibleImageEditProvider, OpenAiCompatibleImageProvider,
    StableDiffusionWebUiImageProvider, pick_image_provider,
};
use a4n_model_discovery::{
    ModelDiscoveryRequest, ModelDiscoveryResult, discover_models_from_request,
};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;
use serde_json::{Value, json};

use crate::contracts::{ImageProviderPayload, ProviderProbeOutput};
use crate::protocol::{
    error_response, result_response, validate_image_request, validate_model_discovery_request,
};

pub(crate) async fn request_preview(
    Json(request): Json<ServiceRequest<ImageProviderPayload>>,
) -> Json<ServiceResponse<Value>> {
    if let Err(error) = validate_image_request(&request) {
        return Json(error_response(&request, error));
    }
    Json(result_response(
        &request,
        image_request_preview(&request.payload),
    ))
}

pub(crate) async fn probe(
    Json(request): Json<ServiceRequest<ImageProviderPayload>>,
) -> Json<ServiceResponse<ProviderProbeOutput>> {
    if let Err(error) = validate_image_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let _provider = pick_image_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        Ok(ProviderProbeOutput {
            ok: true,
            request_preview: image_request_preview(&request.payload)?,
        })
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) async fn discover_models(
    Json(request): Json<ServiceRequest<ModelDiscoveryRequest>>,
) -> Json<ServiceResponse<ModelDiscoveryResult>> {
    if let Err(error) = validate_model_discovery_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = discover_models_from_request(request.payload.clone())
        .await
        .map_err(AppError::ConfigError);
    Json(result_response(&request, result))
}

fn image_request_preview(payload: &ImageProviderPayload) -> Result<Value, AppError> {
    let request = &payload.request;
    match payload.backend.as_str() {
        "openai-compatible-image" => OpenAiCompatibleImageProvider::request_body_preview(request),
        "openai-compatible-image-edit" => {
            OpenAiCompatibleImageEditProvider::request_body_preview(request)
        }
        "openai-chat-image" => Ok(ChatCompletionsImageProvider::request_body_preview(request)),
        "cliproxy-image" => CliProxyImageProvider::request_body_preview(request),
        "stable-diffusion" => StableDiffusionWebUiImageProvider::request_body_preview(request),
        "comfyui" => ComfyUiImageProvider::request_body_preview(request),
        other => Err(AppError::InvalidArgument(format!(
            "未知图片生成后端: {other}"
        ))),
    }
    .map(|body| json!({ "request": body }))
}
