use a4n_agent_runtime_contract::AgentDelegationReviewCommand;

use super::*;

pub(super) async fn spawn_delegation(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_DELEGATION_SPAWN_OPERATION,
        |app, user_id, payload| async move {
            app.spawn_agent(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

pub(super) async fn review_delegation(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_DELEGATION_REVIEW_OPERATION,
        |app, user_id, payload| async move {
            let command: AgentDelegationReviewCommand = decode_payload(payload)?;
            app.review_delegation(user_id, command.input, command.parent_run_id)
                .await
        },
    )
    .await
}
