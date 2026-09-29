use super::*;

pub(super) async fn ensure_main_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_ENSURE_MAIN_OPERATION,
        |app, user_id, payload| async move {
            app.ensure_main_thread(user_id, decode_payload(payload)?)
                .await
        },
    )
    .await
}

pub(super) async fn create_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_CREATE_OPERATION,
        |app, user_id, payload| async move {
            app.create_thread(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

pub(super) async fn fork_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(&state, &headers, request, AGENT_THREAD_FORK_OPERATION, |app, user_id, payload| async move {
        app.fork_thread(user_id, decode_payload(payload)?).await
    })
    .await
}

pub(super) async fn update_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_UPDATE_OPERATION,
        |app, user_id, payload| async move {
            app.update_thread(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

pub(super) async fn archive_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_ARCHIVE_OPERATION,
        |app, user_id, payload| async move {
            app.archive_thread(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

pub(super) async fn list_threads(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(&state, &headers, request, AGENT_THREAD_LIST_OPERATION, |app, user_id, payload| async move {
        app.list_threads(user_id, decode_payload(payload)?).await
    })
    .await
}

pub(super) async fn show_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_SHOW_OPERATION,
        |app, user_id, payload| async move {
            let thread_id: String = decode_payload(payload)?;
            app.show_thread(user_id, &thread_id).await
        },
    )
    .await
}

pub(super) async fn list_messages(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_MESSAGES_OPERATION,
        |app, user_id, payload| async move {
            app.list_messages(user_id, decode_payload(payload)?).await
        },
    )
    .await
}
