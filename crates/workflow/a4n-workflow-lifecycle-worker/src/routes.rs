use super::{
    WorkerState,
    protocol::{WorkerFailure, completed, failed},
};
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerHealth, WorkerHealthStatus,
};
use a4n_workflow_lifecycle_worker_contract::{
    ResumeWorkflowInput, WORKFLOW_LIFECYCLE_CAPABILITY, WORKFLOW_LIFECYCLE_WORKER_CONTRACT_VERSION,
    WorkflowCancellationInput, WorkflowRunIdInput,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};

pub(super) fn router(state: WorkerState) -> Router {
    Router::new()
        .route(WORKER_HEALTH_PATH, get(health))
        .route(WORKER_CAPABILITIES_PATH, get(capabilities))
        .route(
            "/v1/commands/workflow.lifecycle.mark_running",
            post(mark_running),
        )
        .route("/v1/commands/workflow.lifecycle.cancel", post(cancel))
        .route("/v1/commands/workflow.lifecycle.resume", post(resume))
        .with_state(state)
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state.database.ping().await {
        Ok(()) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: [WORKFLOW_LIFECYCLE_CAPABILITY]
            .into_iter()
            .map(|name| WorkerCapability {
                name: name.into(),
                version: WORKFLOW_LIFECYCLE_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            })
            .collect(),
    })
    .into_response()
}

async fn mark_running(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<WorkflowRunIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.mark_running(request)
    })
    .await
}

async fn cancel(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<WorkflowCancellationInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.cancel(request)).await
}

async fn resume(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ResumeWorkflowInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.resume(request)).await
}

async fn command_response<T, O, F, Fut>(
    state: &WorkerState,
    headers: &HeaderMap,
    request: WorkerEnvelope<T>,
    command: F,
) -> Response
where
    T: serde::Serialize + Clone,
    O: serde::Serialize,
    F: FnOnce(WorkerEnvelope<T>) -> Fut,
    Fut: std::future::Future<Output = Result<O, WorkerFailure>>,
{
    if !state.authorized(headers) {
        return unauthorized();
    }
    let response = match command(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
