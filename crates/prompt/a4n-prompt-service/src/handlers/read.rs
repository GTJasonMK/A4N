use crate::app::PromptServiceState;
use crate::protocol::{result_response, validate_service_request};
use a4n_prompt_contract::{
    PromptCategory, PromptEmptyPayload, PromptEntry, PromptListOutput, PromptNamePayload,
    PromptRenderAbOutput, PromptRenderAbPayload, PromptRenderOutput, PromptRenderPayload,
    PromptValidateOutput, PromptValidatePayload, PromptVersionDiffOutput, PromptVersionDiffPayload,
    PromptVersionHistoryOutput,
};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};
use std::collections::HashSet;

pub(crate) async fn list_prompts(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptEmptyPayload>>,
) -> Json<ServiceResponse<PromptListOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let prompts = state.engine.list_all();
    Json(request.ok_response(PromptListOutput { prompts }))
}

pub(crate) async fn get_prompt(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptNamePayload>>,
) -> Json<ServiceResponse<PromptEntry>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    Json(result_response(
        &request,
        state.engine.get(&request.payload.name),
    ))
}

pub(crate) async fn render_prompt(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptRenderPayload>>,
) -> Json<ServiceResponse<PromptRenderOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let payload = &request.payload;
    let snapshot = &payload.assembly;
    let result = if snapshot.prompt_name != payload.template_name {
        Err(a4n_error::AppError::InvalidArgument(
            "提示词装配所属模板与请求模板不匹配".to_string(),
        ))
    } else {
        state
            .engine
            .render_assembled(snapshot, &payload.variables, request.project_id)
    }
    .map(|rendered| PromptRenderOutput { rendered });
    Json(result_response(&request, result))
}

pub(crate) async fn validate_prompt(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptValidatePayload>>,
) -> Json<ServiceResponse<PromptValidateOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let variables: HashSet<String> = request.payload.variables.iter().cloned().collect();
    let result = state
        .engine
        .validate_variables(&request.payload.template_name, &variables)
        .map(|missing| PromptValidateOutput { missing });
    Json(result_response(&request, result))
}

pub(crate) async fn render_prompt_ab(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptRenderAbPayload>>,
) -> Json<ServiceResponse<PromptRenderAbOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let payload = &request.payload;
    let result = state
        .engine
        .render_ab(
            &payload.template_name,
            &payload.variables,
            request.project_id,
        )
        .map(|(current, original)| PromptRenderAbOutput { current, original });
    Json(result_response(&request, result))
}

pub(crate) async fn version_history(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptNamePayload>>,
) -> Json<ServiceResponse<PromptVersionHistoryOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let result = state
        .engine
        .version_history(&request.payload.name)
        .map(|versions| PromptVersionHistoryOutput {
            versions: versions.into_iter().map(Into::into).collect(),
        });
    Json(result_response(&request, result))
}

pub(crate) async fn version_diff(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptVersionDiffPayload>>,
) -> Json<ServiceResponse<PromptVersionDiffOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let payload = &request.payload;
    let result = state
        .engine
        .diff(&payload.name, &payload.version_a, &payload.version_b)
        .map(|diff| PromptVersionDiffOutput { diff });
    Json(result_response(&request, result))
}

pub(crate) async fn categories(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptEmptyPayload>>,
) -> Json<ServiceResponse<Vec<PromptCategory>>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    Json(result_response(&request, state.engine.categories()))
}
