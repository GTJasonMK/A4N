use crate::contracts::{
    VideoDeleteOutput, VideoJobOutput, VideoPollPayload, VideoProviderPayload, VideoStatusOutput,
};
use crate::events::video_generation_event_stream;
use crate::protocol::{
    result_response, service_error, validate_video_poll_request, validate_video_provider_request,
};
use crate::providers::{video_delete_output, video_job_output, video_status_output};
use a4n_domain::ProviderError as AppError;
use a4n_model_discovery::discover_models_from_request;
use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_service_protocol::{ServiceRequest, ServiceResponse, VIDEO_SERVICE_NAME};
use a4n_sidecar_bootstrap::{failed_service_event_response, service_event_ndjson_response};
use a4n_video::VideoProvider;
use a4n_video_providers::{pick_video_provider, video_provider_request_body_preview};
use axum::{Json, response::IntoResponse};
use serde_json::Value;
use std::future::Future;

pub(crate) async fn generate_events(
    Json(request): Json<ServiceRequest<VideoProviderPayload>>,
) -> impl IntoResponse {
    let task_id = request.request_id.clone();
    if let Err(error) = validate_video_provider_request(&request) {
        return failed_service_event_response(task_id, error);
    }
    service_event_ndjson_response(video_generation_event_stream(request))
}

async fn execute_video_job_operation<T, F, Fut>(
    request: ServiceRequest<VideoPollPayload>,
    operation: F,
) -> Json<ServiceResponse<T>>
where
    F: FnOnce(Box<dyn VideoProvider>, String) -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    if let Err(error) = validate_video_poll_request(&request) {
        return Json(request.error_response(error));
    }
    let result = async {
        let provider = pick_video_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        operation(provider, request.payload.job_id.clone()).await
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) async fn poll(
    Json(request): Json<ServiceRequest<VideoPollPayload>>,
) -> Json<ServiceResponse<VideoJobOutput>> {
    execute_video_job_operation(request, |provider, job_id| async move {
        provider.poll(&job_id).await.map(video_job_output)
    })
    .await
}

pub(crate) async fn status(
    Json(request): Json<ServiceRequest<VideoPollPayload>>,
) -> Json<ServiceResponse<VideoStatusOutput>> {
    execute_video_job_operation(request, |provider, job_id| async move {
        provider.status(&job_id).await.map(video_status_output)
    })
    .await
}

pub(crate) async fn delete(
    Json(request): Json<ServiceRequest<VideoPollPayload>>,
) -> Json<ServiceResponse<VideoDeleteOutput>> {
    execute_video_job_operation(request, |provider, job_id| async move {
        provider.delete(&job_id).await.map(video_delete_output)
    })
    .await
}

pub(crate) async fn request_preview(
    Json(request): Json<ServiceRequest<VideoProviderPayload>>,
) -> Json<ServiceResponse<Value>> {
    if let Err(error) = validate_video_provider_request(&request) {
        return Json(request.error_response(error));
    }
    let result = video_provider_request_body_preview(
        &request.payload.backend,
        &request.payload.base_url,
        &request.payload.request,
    );
    Json(result_response(&request, result))
}

pub(crate) async fn discover_models(
    Json(request): Json<ServiceRequest<ModelDiscoveryRequest>>,
) -> Json<ServiceResponse<ModelDiscoveryResult>> {
    if let Err(error) = request
        .validate_known_provider_request(VIDEO_SERVICE_NAME, request.payload.api_key.as_deref())
    {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let result = discover_models_from_request(request.payload)
        .await
        .map_err(AppError::Configuration);
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}
