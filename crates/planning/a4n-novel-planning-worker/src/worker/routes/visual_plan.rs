use super::super::responses::{completed, failed, unauthorized};
use crate::*;

pub(crate) async fn save_visual_plan_draft(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SaveChapterVisualPlanDraftInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.save_visual_plan_draft(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn confirm_visual_plan(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterVisualPlanRecordInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.confirm_visual_plan(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn discard_visual_plan(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterVisualPlanRecordInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.discard_visual_plan(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn get_visual_plan(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterVisualPlanSourceInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.get_visual_plan(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn read_visual_plan(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ReadChapterVisualPlanInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.read_visual_plan(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn validate_visual_plan(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ValidateChapterVisualPlanInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.validate_visual_plan(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}
