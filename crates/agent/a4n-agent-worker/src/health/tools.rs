use super::*;

pub(super) async fn search_tools(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_TOOL_SEARCH_OPERATION,
        |app, _user_id, payload| async move {
            app.search_tools_for_worker(decode_payload(payload)?)
        },
    )
    .await
}

pub(super) async fn describe_tool(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_TOOL_DESCRIBE_OPERATION,
        |app, _user_id, payload| async move {
            app.describe_tool_for_worker(decode_payload(payload)?)
        },
    )
    .await
}
