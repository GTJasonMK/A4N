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
        capabilities: vec![
            WorkerCapability {
                name: SHOOTING_SCRIPT_CURRENT_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_UNIT_PROGRESS_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_EDIT_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_EDIT_PROGRESS_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_DELETE_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_PUBLISH_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VISUAL_PLAN_PERSISTENCE_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn current(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterShootingScriptSourceInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.current(request.clone()).await {
        Ok(payload) => completed(&request, OptionalWorkerPayload { value: payload }),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn unit_progress(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterShootingScriptSourceInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.unit_progress(request.clone()).await {
        Ok(payload) => completed(&request, OptionalWorkerPayload { value: payload }),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn edit(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EditChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.edit(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn edit_progress(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EditChapterShootingScriptProgressInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.edit_progress(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn delete(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.delete(request.clone()).await {
        Ok(()) => completed(&request, ()),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn publish(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PublishChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.publish(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn save_visual_plan_draft(
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

async fn confirm_visual_plan(
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

async fn discard_visual_plan(
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

async fn get_visual_plan(
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

async fn read_visual_plan(
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

async fn validate_visual_plan(
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
