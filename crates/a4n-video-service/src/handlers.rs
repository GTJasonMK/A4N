use crate::contracts::*;
use crate::events::{failed_generation_event, video_generation_event_stream};
use crate::protocol::{
    error_response, result_response, validate_model_discovery_request, validate_video_poll_request,
    validate_video_provider_request,
};
use crate::providers::{
    video_delete_output, video_generation_output, video_job_output, video_request_preview,
    video_status_output,
};
use a4n_core::error::AppError;
use a4n_model_discovery::{
    ModelDiscoveryRequest, ModelDiscoveryResult, discover_models_from_request,
};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use a4n_sidecar_bootstrap::{service_event_ndjson_response, single_service_event_stream};
use a4n_video_providers::pick_video_provider;
use axum::{Json, response::IntoResponse};
use serde_json::Value;

pub(crate) async fn generate(
    Json(request): Json<ServiceRequest<VideoProviderPayload>>,
) -> Json<ServiceResponse<VideoGenerationOutput>> {
    if let Err(error) = validate_video_provider_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let provider = pick_video_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        provider
            .generate(
                request.payload.request.clone(),
                request.payload.provider_idempotency_key_sha256.as_deref(),
            )
            .await
            .map(video_generation_output)
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) async fn generate_events(
    Json(request): Json<ServiceRequest<VideoProviderPayload>>,
) -> impl IntoResponse {
    let task_id = request.request_id.clone();
    if let Err(error) = validate_video_provider_request(&request) {
        return service_event_ndjson_response(
            single_service_event_stream(failed_generation_event(task_id, 1, error)),
            "video generation",
        );
    }
    service_event_ndjson_response(video_generation_event_stream(request), "video generation")
}

pub(crate) async fn poll(
    Json(request): Json<ServiceRequest<VideoPollPayload>>,
) -> Json<ServiceResponse<VideoJobOutput>> {
    if let Err(error) = validate_video_poll_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let provider = pick_video_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        provider
            .poll(&request.payload.job_id)
            .await
            .map(video_job_output)
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) async fn status(
    Json(request): Json<ServiceRequest<VideoPollPayload>>,
) -> Json<ServiceResponse<VideoStatusOutput>> {
    if let Err(error) = validate_video_poll_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let provider = pick_video_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        provider
            .status(&request.payload.job_id)
            .await
            .map(video_status_output)
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) async fn delete(
    Json(request): Json<ServiceRequest<VideoPollPayload>>,
) -> Json<ServiceResponse<VideoDeleteOutput>> {
    if let Err(error) = validate_video_poll_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let provider = pick_video_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        provider
            .delete(&request.payload.job_id)
            .await
            .map(video_delete_output)
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) async fn request_preview(
    Json(request): Json<ServiceRequest<VideoProviderPayload>>,
) -> Json<ServiceResponse<Value>> {
    if let Err(error) = validate_video_provider_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = video_request_preview(&request.payload);
    Json(result_response(&request, result))
}

pub(crate) async fn probe(
    Json(request): Json<ServiceRequest<VideoProviderPayload>>,
) -> Json<ServiceResponse<ProviderProbeOutput>> {
    if let Err(error) = validate_video_provider_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let _provider = pick_video_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        let request_preview = video_request_preview(&request.payload)?;
        Ok(ProviderProbeOutput {
            ok: true,
            request_preview,
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
