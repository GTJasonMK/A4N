use crate::app::PromptServiceState;
use crate::contracts::{
    CategoryInfo, PromptDependencyGraphOutput, PromptListOutput, PromptNamePayload,
    PromptRenderAbOutput, PromptRenderOutput, PromptRenderPayload, PromptValidateOutput,
    PromptValidatePayload, PromptVersionDiffOutput, PromptVersionDiffPayload,
    PromptVersionHistoryOutput, PromptVersionPayload,
};
use crate::protocol::{error_response, ok_response, result_response, validate_service_request};
use a4n_core::error::AppError;
use a4n_prompt::engine::{PromptEngine, PromptEntry};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};
use std::collections::HashSet;

pub(crate) async fn list_prompts(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<serde_json::Value>>,
) -> Json<ServiceResponse<PromptListOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let mut prompts = state.engine.list_all();
    prompts.sort_by(|left, right| left.name.cmp(&right.name));
    Json(ok_response(&request, PromptListOutput { prompts }))
}

pub(crate) async fn get_prompt(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptNamePayload>>,
) -> Json<ServiceResponse<PromptEntry>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
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
        return Json(error_response(&request, error));
    }
    let payload = &request.payload;
    let result = state
        .engine
        .render(
            &payload.template_name,
            &payload.variables,
            payload.project_id,
        )
        .map(|rendered| PromptRenderOutput { rendered });
    Json(result_response(&request, result))
}

pub(crate) async fn validate_prompt(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptValidatePayload>>,
) -> Json<ServiceResponse<PromptValidateOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
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
    Json(request): Json<ServiceRequest<PromptRenderPayload>>,
) -> Json<ServiceResponse<PromptRenderAbOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = &request.payload;
    let result = state
        .engine
        .render_ab(
            &payload.template_name,
            &payload.variables,
            payload.project_id,
        )
        .map(|(version_a, version_b)| PromptRenderAbOutput {
            version_a,
            version_b,
        });
    Json(result_response(&request, result))
}

pub(crate) async fn version_history(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptNamePayload>>,
) -> Json<ServiceResponse<PromptVersionHistoryOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .engine
        .version_history(&request.payload.name)
        .map(|versions| PromptVersionHistoryOutput { versions });
    Json(result_response(&request, result))
}

pub(crate) async fn version_get(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptVersionPayload>>,
) -> Json<ServiceResponse<PromptEntry>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = &request.payload;
    Json(result_response(
        &request,
        state.engine.get_version(&payload.name, &payload.version),
    ))
}

pub(crate) async fn version_diff(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<PromptVersionDiffPayload>>,
) -> Json<ServiceResponse<PromptVersionDiffOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = &request.payload;
    let result = state
        .engine
        .diff(&payload.name, &payload.version_a, &payload.version_b)
        .map(|diff| PromptVersionDiffOutput { diff });
    Json(result_response(&request, result))
}

pub(crate) async fn dependency_graph(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<serde_json::Value>>,
) -> Json<ServiceResponse<PromptDependencyGraphOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    Json(ok_response(
        &request,
        PromptDependencyGraphOutput {
            graph: state.engine.dependency_graph(),
        },
    ))
}

pub(crate) async fn categories(
    State(state): State<PromptServiceState>,
    Json(request): Json<ServiceRequest<serde_json::Value>>,
) -> Json<ServiceResponse<Vec<CategoryInfo>>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    Json(result_response(
        &request,
        prompt_categories(state.engine.as_ref()),
    ))
}

fn prompt_categories(engine: &dyn PromptEngine) -> Result<Vec<CategoryInfo>, AppError> {
    let registry = engine.registry();
    let registry = registry.read();
    let mut categories = Vec::with_capacity(registry.categories().len());
    for (key, value) in registry.categories() {
        categories.push(CategoryInfo {
            key: key.clone(),
            label: value.label.clone(),
            order: value
                .order
                .ok_or_else(|| AppError::ConfigError(format!("提示词分类 {key} 缺少 order")))?,
            description: value.description.clone(),
        });
    }
    categories.sort_by_key(|category| (category.order, category.key.clone()));
    Ok(categories)
}
