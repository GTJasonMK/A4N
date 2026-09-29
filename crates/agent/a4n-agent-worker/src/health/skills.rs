use a4n_agent_runtime_contract::AgentSkillCapabilityCommand;

use super::*;

pub(super) async fn list_skill_settings(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_SKILL_SETTINGS_LIST_OPERATION,
        |app, user_id, _| async move { app.list_skill_settings(user_id).await },
    )
    .await
}

pub(super) async fn update_skill_configuration(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_SKILL_SETTINGS_UPDATE_OPERATION,
        |app, user_id, payload| async move {
            app.update_skill_configuration(user_id, decode_payload(payload)?)
                .await
        },
    )
    .await
}

pub(super) async fn search_skills(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_SKILL_SEARCH_OPERATION,
        |app, user_id, payload| async move {
            let command: AgentSkillCapabilityCommand<Value> = decode_payload(payload)?;
            app.search_skills(user_id, &command.capability, decode_payload(command.input)?)
                .await
        },
    )
    .await
}

pub(super) async fn load_skill(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_SKILL_LOAD_OPERATION,
        |app, user_id, payload| async move {
            let command: AgentSkillCapabilityCommand<Value> = decode_payload(payload)?;
            app.load_skill(user_id, &command.capability, decode_payload(command.input)?)
                .await
        },
    )
    .await
}
