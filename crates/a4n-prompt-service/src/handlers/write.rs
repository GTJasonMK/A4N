use crate::app::PromptServiceState;
use crate::contracts::{
    CountOutput, PromptHydrateOverridesPayload, PromptNamePayload, PromptProjectOverridePayload,
    PromptUpdateManyPayload, PromptUpdatePayload,
};
use crate::protocol::{error_response, result_response, validate_service_request};
use a4n_core::error::AppError;
use a4n_prompt::engine::{PromptEngine, PromptEntry};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};

pub(crate) async fn hydrate_overrides(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptHydrateOverridesPayload>>,
) -> Json<ServiceResponse<CountOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let overrides = request
        .payload
        .overrides
        .iter()
        .map(|item| (item.project_id, item.name.clone(), item.content.clone()))
        .collect::<Vec<_>>();
    let count = overrides.len();
    let result = state
        .engine
        .hydrate_project_overrides(overrides)
        .map(|_| CountOutput { count });
    Json(result_response(&request, result))
}

pub(crate) async fn set_project_override(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptProjectOverridePayload>>,
) -> Json<ServiceResponse<serde_json::Value>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = match payload.content {
        Some(content) => state
            .engine
            .set_project_override(payload.project_id, &payload.name, content)
            .await
            .map(|_| serde_json::json!({})),
        None => Err(AppError::InvalidArgument(
            "prompt override content 不能为空".to_string(),
        )),
    };
    Json(result_response(&request, result))
}

pub(crate) async fn remove_project_override(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptProjectOverridePayload>>,
) -> Json<ServiceResponse<serde_json::Value>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = &request.payload;
    let result = state
        .engine
        .remove_project_override(payload.project_id, &payload.name)
        .await
        .map(|_| serde_json::json!({}));
    Json(result_response(&request, result))
}

pub(crate) async fn load_defaults(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<serde_json::Value>>,
) -> Json<ServiceResponse<CountOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .engine
        .load_defaults()
        .await
        .map(|count| CountOutput { count });
    Json(result_response(&request, result))
}

pub(crate) async fn update_prompt(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptUpdatePayload>>,
) -> Json<ServiceResponse<PromptEntry>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = &request.payload;
    let result = state
        .engine
        .update(&payload.name, payload.content.clone(), &payload.author)
        .await;
    Json(result_response(&request, result))
}

pub(crate) async fn update_many_prompts(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptUpdateManyPayload>>,
) -> Json<ServiceResponse<Vec<PromptEntry>>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = &request.payload;
    let updates = payload
        .updates
        .iter()
        .map(|update| (update.name.clone(), update.content.clone()))
        .collect::<Vec<_>>();
    let result = state.engine.update_many(updates, &payload.author).await;
    Json(result_response(&request, result))
}

pub(crate) async fn reset_prompt(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptNamePayload>>,
) -> Json<ServiceResponse<serde_json::Value>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .engine
        .reset(&request.payload.name)
        .await
        .map(|_| serde_json::json!({}));
    Json(result_response(&request, result))
}
