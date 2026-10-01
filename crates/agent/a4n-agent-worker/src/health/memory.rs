use a4n_agent_runtime_contract::AgentMemoryListInput;

use super::*;

pub(super) async fn list_memories(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_MEMORY_LIST_OPERATION,
        |app, user_id, payload| async move {
            let input: AgentMemoryListInput = decode_payload(payload)?;
            app.list_memories(user_id, input.scope_kind, input.scope_id)
                .await
        },
    )
    .await
}

pub(super) async fn create_memory(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_MEMORY_CREATE_OPERATION,
        |app, user_id, payload| async move {
            app.create_memory(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

pub(super) async fn retract_memory(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_MEMORY_RETRACT_OPERATION,
        |app, user_id, payload| async move {
            app.retract_memory(user_id, decode_payload(payload)?).await
        },
    )
    .await
}
