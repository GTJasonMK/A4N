use super::auth::authenticated_user;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_agent_application::agent::AgentExternalTaskWaitSnapshot;
use a4n_api_contract::agent::{
    AGENT_EVENTS_DEFAULT_LIMIT, AGENT_EVENTS_MAX_LIMIT, AGENT_THREADS_DEFAULT_LIMIT,
    AGENT_THREADS_MAX_LIMIT, AGENT_TOOLS_DEFAULT_LIMIT, AGENT_TOOLS_MAX_LIMIT, AgentDelegationView,
    AgentEvent, AgentEventsInput, AgentMemoryInput, AgentMemoryScopeKind, AgentMemoryView,
    AgentMessageAccepted, AgentMessageView, AgentRunView, AgentThreadSnapshot, AgentThreadView,
    AgentToolDescriptor, AgentToolSearchResult, ArchiveAgentThreadInput, ControlAgentRunInput,
    ControlChildAgentInput, CreateAgentThreadInput, DescribeAgentToolInput,
    EnsureMainAgentThreadInput, ForkAgentThreadInput, ListAgentMessagesInput,
    ListAgentThreadsInput, ResolveAgentApprovalInput, RetractAgentMemoryInput,
    SearchAgentToolsInput, SendAgentMessageInput, SendAgentMessageToChildInput, SpawnAgentInput,
    UpdateAgentThreadInput,
};
use a4n_api_contract::chapter_visual_plan::VisualPlanProjection;
use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, ModelCapabilities};
use a4n_runtime::host_api::agent_runtime as agent_actions;
use a4n_worker_contract::{
    AGENT_WORKER_AUTH_HEADER, AgentHostCommandPayload, AgentRunPayload,
    AgentWorkerCancelGenerationRequest, AgentWorkerCancelWorkflowRequest, AgentWorkerChatRequest,
    AgentWorkerClaimRequest, AgentWorkerEditorProjectionRequest, AgentWorkerLeaseRenewRequest,
    AgentWorkerLlmCapabilitiesRequest, AgentWorkerPackageExecutionResponse,
    AgentWorkerRunContextWire, AgentWorkerWaitRequest, WorkerRequest,
};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::{Stream, StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::time::Duration;

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub(super) async fn claim_worker(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerClaimRequest>,
) -> Result<Response, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    match agent_actions::claim_agent_worker_request(state.runtime(), &input.worker_id).await? {
        Some(request) => Ok(Json(request).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

pub(super) async fn load_agent_context(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerRequest<AgentRunPayload>>,
) -> Result<Json<AgentWorkerRunContextWire>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    request
        .validate_agent_run()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::load_agent_run_context(state.runtime(), &request)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn renew_agent_lease(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerLeaseRenewRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    input
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    agent_actions::renew_agent_worker_lease(state.runtime(), &input).await?;
    Ok(Json(Value::Null))
}

pub(super) async fn command(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerRequest<AgentHostCommandPayload>>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    request
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    request
        .payload
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let user_id = agent_actions::authorize_agent_host_command(state.runtime(), &request).await?;
    let command = request.payload.command.clone();
    let arguments = request.payload.arguments_with_runtime_snapshot();
    super::commands::dispatch_runtime_command_value(state.runtime(), user_id, arguments, command)
        .await
        .map(Json)
        .map_err(|error| {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Agent Host command 执行失败")
                .to_string();
            ApiError::internal("agent_host_command_failed", message, true)
        })
}

pub(super) async fn agent_package_execution_enabled(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<AgentWorkerPackageExecutionResponse>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    let enabled = agent_actions::agent_package_execution_enabled(state.runtime()).await?;
    Ok(Json(AgentWorkerPackageExecutionResponse { enabled }))
}

pub(super) async fn cancel_agent_generation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerCancelGenerationRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::cancel_agent_owned_generation_task(
        state.runtime(),
        input.user_id,
        &input.task_id,
        input.attempt,
    )
    .await?;
    Ok(Json(Value::Null))
}

pub(super) async fn cancel_agent_workflow(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerCancelWorkflowRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::cancel_agent_owned_workflow(
        state.runtime(),
        input.workflow_run_id,
        &input.reason,
    )
    .await?;
    Ok(Json(Value::Null))
}

pub(super) async fn read_agent_wait(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerWaitRequest>,
) -> Result<Json<Value>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    let value = agent_actions::read_external_task_wait(
        state.runtime(),
        AgentExternalTaskWaitSnapshot {
            run_id: input.run_id,
            owner_user_id: input.owner_user_id,
            scope: input.scope,
            execution_id: input.execution_id,
            tool_name: input.tool_name,
            arguments_json: input.arguments_json,
            duration_exhausted: input.duration_exhausted,
        },
        input.runtime_config_snapshot,
    )
    .await?;
    Ok(Json(value))
}

pub(super) async fn agent_llm_capabilities(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerLlmCapabilitiesRequest>,
) -> Result<Json<ModelCapabilities>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::agent_llm_capabilities(
        state.runtime(),
        input.user_id,
        input.snapshot,
        input.config_id,
    )
    .await
    .map(Json)
    .map_err(Into::into)
}

pub(super) async fn agent_chat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerChatRequest>,
) -> Result<Response, ApiError> {
    authorize_worker_request(&state, &headers)?;
    let request = serde_json::from_value::<ChatRequest>(input.request)
        .map_err(|error| ApiError::bad_request(format!("Agent LLM request 无效: {error}")))?;
    let chunks = agent_actions::agent_chat_stream(
        state.runtime(),
        input.user_id,
        input.snapshot,
        input.config_id,
        request,
    )
    .await?;
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-transform"),
    );
    response_headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
    Ok((
        response_headers,
        Sse::new(agent_chat_sse_stream(chunks)).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(5))
                .text("keep-alive"),
        ),
    )
        .into_response())
}

pub(super) async fn project_agent_editor(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentWorkerEditorProjectionRequest>,
) -> Result<Json<VisualPlanProjection>, ApiError> {
    authorize_worker_request(&state, &headers)?;
    agent_actions::project_agent_editor_draft(state.runtime(), &input.plan, &input.view)
        .map(Json)
        .map_err(Into::into)
}

fn agent_chat_sse_stream<S>(
    chunks: S,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static
where
    S: Stream<Item = Result<ChatChunk, AppError>> + Send + 'static,
{
    stream::unfold(
        (Box::pin(chunks), false),
        |(mut chunks, finished)| async move {
            if finished {
                return None;
            }
            match chunks.as_mut().next().await {
                Some(Ok(chunk)) => Some((Ok(chat_chunk_event(chunk)), (chunks, false))),
                Some(Err(error)) => Some((Ok(chat_error_event(error)), (chunks, true))),
                None => None,
            }
        },
    )
}

fn chat_chunk_event(chunk: ChatChunk) -> Event {
    Event::default()
        .event("chunk")
        .data(serde_json::to_string(&chunk).expect("ChatChunk must remain serializable"))
}

fn chat_error_event(error: AppError) -> Event {
    Event::default()
        .event("error")
        .data(serde_json::json!({ "message": error.to_string() }).to_string())
}

fn authorize_worker_request(state: &ServerState, headers: &HeaderMap) -> Result<(), ApiError> {
    let provided = headers
        .get(AGENT_WORKER_AUTH_HEADER)
        .and_then(|value| value.to_str().ok());
    if state.agent_worker_token_matches(provided) {
        Ok(())
    } else {
        Err(ApiError::unauthorized("Agent worker 身份验证失败"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ThreadListQuery {
    capability: Option<String>,
    #[serde(default)]
    include_archived: bool,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentEventsQuery {
    after_sequence: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentToolSearchQuery {
    query: Option<String>,
    domain: Option<String>,
    effect: Option<String>,
    capability: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentMessagesQuery {
    before: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentMemoriesQuery {
    scope_kind: Option<AgentMemoryScopeKind>,
    scope_id: Option<String>,
}

pub(super) async fn list_threads(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<ThreadListQuery>,
) -> Result<Json<Vec<AgentThreadView>>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let input = ListAgentThreadsInput {
        capability: optional_query_text(query.capability, "capability")?,
        include_archived: query.include_archived,
        limit: Some(parse_limit(
            query.limit.as_deref(),
            AGENT_THREADS_DEFAULT_LIMIT,
            AGENT_THREADS_MAX_LIMIT,
        )?),
    };
    agent_actions::list_threads(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn ensure_main_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<EnsureMainAgentThreadInput>,
) -> Result<Json<AgentThreadView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    agent_actions::ensure_main_thread(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn create_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateAgentThreadInput>,
) -> Result<(StatusCode, Json<AgentThreadView>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let thread = agent_actions::create_thread(state.runtime(), user.id, input).await?;
    Ok((StatusCode::CREATED, Json(thread)))
}

pub(super) async fn fork_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(mut input): Json<ForkAgentThreadInput>,
) -> Result<(StatusCode, Json<AgentThreadView>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&thread_id, &input.thread_id, "thread_id")?;
    input.thread_id = required_path_id(&thread_id, "thread_id")?.to_string();
    let thread = agent_actions::fork_thread(state.runtime(), user.id, input).await?;
    Ok((StatusCode::CREATED, Json(thread)))
}

pub(super) async fn update_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(mut input): Json<UpdateAgentThreadInput>,
) -> Result<Json<AgentThreadView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&thread_id, &input.thread_id, "thread_id")?;
    input.thread_id = required_path_id(&thread_id, "thread_id")?.to_string();
    agent_actions::update_thread(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn archive_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(mut input): Json<ArchiveAgentThreadInput>,
) -> Result<Json<AgentThreadView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&thread_id, &input.thread_id, "thread_id")?;
    input.thread_id = required_path_id(&thread_id, "thread_id")?.to_string();
    agent_actions::archive_thread(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn show_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
) -> Result<Json<AgentThreadSnapshot>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let thread_id = required_path_id(&thread_id, "thread_id")?;
    agent_actions::show_thread(state.runtime(), user.id, thread_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn list_messages(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Query(query): Query<AgentMessagesQuery>,
) -> Result<Json<Vec<AgentMessageView>>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let thread_id = required_path_id(&thread_id, "thread_id")?.to_string();
    let input = ListAgentMessagesInput {
        thread_id,
        before: optional_query_text(query.before, "before")?,
        limit: Some(parse_limit(
            query.limit.as_deref(),
            AGENT_EVENTS_DEFAULT_LIMIT,
            AGENT_EVENTS_MAX_LIMIT,
        )?),
    };
    agent_actions::list_messages(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn send_message(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(input): Json<SendAgentMessageInput>,
) -> Result<(StatusCode, Json<AgentMessageAccepted>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&thread_id, &input.thread_id, "thread_id")?;
    let accepted =
        agent_actions::send_message_with_admission(state.runtime(), user.id, input, true).await?;
    Ok((StatusCode::ACCEPTED, Json(accepted)))
}

pub(super) async fn control_run(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
    Json(input): Json<ControlAgentRunInput>,
) -> Result<Json<AgentRunView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&run_id, &input.run_id, "run_id")?;
    agent_actions::control_run(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn show_run(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
) -> Result<Json<AgentRunView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let run_id = required_path_id(&run_id, "run_id")?;
    agent_actions::show_run(state.runtime(), user.id, run_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn spawn_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(parent_run_id): Path<String>,
    Json(mut input): Json<SpawnAgentInput>,
) -> Result<Json<AgentDelegationView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&parent_run_id, &input.parent_run_id, "parent_run_id")?;
    input.parent_run_id = required_path_id(&parent_run_id, "parent_run_id")?.to_string();
    agent_actions::spawn_agent_with_admission(state.runtime(), user.id, input, true)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn send_message_to_child(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(delegation_id): Path<String>,
    Json(mut input): Json<SendAgentMessageToChildInput>,
) -> Result<(StatusCode, Json<AgentMessageAccepted>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&delegation_id, &input.delegation_id, "delegation_id")?;
    input.delegation_id = required_path_id(&delegation_id, "delegation_id")?.to_string();
    let accepted =
        agent_actions::send_message_to_child_with_admission(state.runtime(), user.id, input, true)
            .await?;
    Ok((StatusCode::ACCEPTED, Json(accepted)))
}

pub(super) async fn control_child_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(delegation_id): Path<String>,
    Json(mut input): Json<ControlChildAgentInput>,
) -> Result<Json<AgentRunView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&delegation_id, &input.delegation_id, "delegation_id")?;
    input.delegation_id = required_path_id(&delegation_id, "delegation_id")?.to_string();
    agent_actions::control_child_agent(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Query(query): Query<AgentEventsQuery>,
) -> Result<Response, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let thread_id = required_path_id(&thread_id, "thread_id")?.to_string();
    let query_cursor = parse_optional_sequence(query.after_sequence.as_deref())?;
    let limit = parse_limit(
        query.limit.as_deref(),
        AGENT_EVENTS_DEFAULT_LIMIT,
        AGENT_EVENTS_MAX_LIMIT,
    )?;

    if !accepts_event_stream(&headers)? {
        let input = AgentEventsInput {
            thread_id: thread_id.clone(),
            after_sequence: query_cursor,
            limit: Some(limit),
        };
        let events = agent_actions::events(state.runtime(), user.id, input).await?;
        validate_event_batch(&thread_id, query_cursor.unwrap_or(0), &events)?;
        return Ok(Json(events).into_response());
    }

    let cursor = resolve_event_cursor(&headers, query_cursor)?;
    let initial = agent_actions::events(
        state.runtime(),
        user.id,
        AgentEventsInput {
            thread_id: thread_id.clone(),
            after_sequence: cursor,
            limit: Some(limit),
        },
    )
    .await?;
    validate_event_batch(&thread_id, cursor.unwrap_or(0), &initial)?;

    let stream = agent_event_stream(AgentSseState {
        state,
        user_id: user.id,
        thread_id,
        limit,
        cursor: cursor.unwrap_or(0),
        pending: initial.into(),
    });
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-transform"),
    );
    response_headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
    Ok((
        response_headers,
        Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(5))
                .text("keep-alive"),
        ),
    )
        .into_response())
}

pub(super) async fn search_tools(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<AgentToolSearchQuery>,
) -> Result<Json<AgentToolSearchResult>, ApiError> {
    authenticated_user(&state, &headers).await?;
    let input = SearchAgentToolsInput {
        query: optional_query_text(query.query, "query")?,
        domain: optional_query_text(query.domain, "domain")?,
        effect: optional_query_text(query.effect, "effect")?,
        capability: optional_query_text(query.capability, "capability")?,
        limit: Some(parse_limit(
            query.limit.as_deref(),
            AGENT_TOOLS_DEFAULT_LIMIT,
            AGENT_TOOLS_MAX_LIMIT,
        )?),
    };
    agent_actions::search_tools(state.runtime(), input)
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn describe_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<AgentToolDescriptor>, ApiError> {
    authenticated_user(&state, &headers).await?;
    let name = required_path_id(&name, "name")?.to_string();
    agent_actions::describe_tool(state.runtime(), DescribeAgentToolInput { name })
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn list_memories(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<AgentMemoriesQuery>,
) -> Result<Json<Vec<AgentMemoryView>>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let scope_id = query
        .scope_id
        .map(|value| optional_query_text(Some(value), "scope_id"))
        .transpose()?
        .flatten();
    agent_actions::list_memories(state.runtime(), user.id, query.scope_kind, scope_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn create_memory(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AgentMemoryInput>,
) -> Result<(StatusCode, Json<AgentMemoryView>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let memory = agent_actions::create_memory(state.runtime(), user.id, input).await?;
    Ok((StatusCode::CREATED, Json(memory)))
}

pub(super) async fn retract_memory(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(memory_id): Path<String>,
    Json(mut input): Json<RetractAgentMemoryInput>,
) -> Result<Json<AgentMemoryView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&memory_id, &input.memory_id, "memory_id")?;
    input.memory_id = required_path_id(&memory_id, "memory_id")?.to_string();
    agent_actions::retract_memory(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn resolve_approval(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(approval_id): Path<String>,
    Json(input): Json<ResolveAgentApprovalInput>,
) -> Result<Json<a4n_api_contract::agent::AgentApprovalView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    require_matching_id(&approval_id, &input.approval_id, "approval_id")?;
    agent_actions::resolve_approval(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

struct AgentSseState {
    state: ServerState,
    user_id: i64,
    thread_id: String,
    limit: u32,
    cursor: u64,
    pending: VecDeque<AgentEvent>,
}

fn agent_event_stream(
    state: AgentSseState,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    stream::unfold(state, |mut state| async move {
        loop {
            if let Some(event) = state.pending.pop_front() {
                state.cursor = event.sequence;
                return Some((Ok(to_sse_event(&event)), state));
            }

            tokio::time::sleep(EVENT_POLL_INTERVAL).await;
            let input = AgentEventsInput {
                thread_id: state.thread_id.clone(),
                after_sequence: Some(state.cursor),
                limit: Some(state.limit),
            };
            let events =
                match agent_actions::events(state.state.runtime(), state.user_id, input).await {
                    Ok(events) => events,
                    Err(error) => {
                        tracing::warn!(
                            thread_id = %state.thread_id,
                            cursor = state.cursor,
                            error = %error,
                            "Agent SSE durable replay failed; closing stream"
                        );
                        return None;
                    }
                };
            if let Err(error) = validate_event_batch(&state.thread_id, state.cursor, &events) {
                tracing::error!(
                    thread_id = %state.thread_id,
                    cursor = state.cursor,
                    error = %error,
                    "Agent SSE durable event invariant failed; closing stream"
                );
                return None;
            }
            state.pending.extend(events);
        }
    })
}

fn to_sse_event(event: &AgentEvent) -> Event {
    let (id, data) = agent_event_payload(event);
    Event::default().event("agent_event").id(id).data(data)
}

fn agent_event_payload(event: &AgentEvent) -> (String, String) {
    (
        event.sequence.to_string(),
        serde_json::to_string(event).expect("AgentEvent must remain serializable"),
    )
}

fn accepts_event_stream(headers: &HeaderMap) -> Result<bool, ApiError> {
    let Some(value) = headers.get(header::ACCEPT) else {
        return Ok(false);
    };
    let raw = value
        .to_str()
        .map_err(|_| ApiError::bad_request("Accept 头不是有效 ASCII 字符串"))?;
    Ok(raw.split(',').any(|media_range| {
        media_range
            .split(';')
            .next()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
    }))
}

fn resolve_event_cursor(
    headers: &HeaderMap,
    query_cursor: Option<u64>,
) -> Result<Option<u64>, ApiError> {
    let Some(value) = headers.get(HeaderName::from_static("last-event-id")) else {
        return Ok(query_cursor);
    };
    let raw = value
        .to_str()
        .map_err(|_| ApiError::bad_request("Last-Event-ID 不是有效 ASCII 字符串"))?;
    let header_cursor = parse_sequence(raw, "Last-Event-ID")?;
    if query_cursor.is_some_and(|cursor| cursor != header_cursor) {
        return Err(ApiError::bad_request(
            "Last-Event-ID 必须与 after_sequence 一致",
        ));
    }
    Ok(Some(header_cursor))
}

fn parse_optional_sequence(raw: Option<&str>) -> Result<Option<u64>, ApiError> {
    raw.map(|value| parse_sequence(value, "after_sequence"))
        .transpose()
}

fn parse_sequence(raw: &str, field: &str) -> Result<u64, ApiError> {
    if raw.trim() != raw || raw.is_empty() {
        return Err(ApiError::bad_request(format!("{field} 必须是非负整数")));
    }
    raw.parse::<u64>()
        .map_err(|_| ApiError::bad_request(format!("{field} 必须是非负整数")))
}

fn parse_limit(raw: Option<&str>, default: u32, max: u32) -> Result<u32, ApiError> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    let limit = raw
        .parse::<u32>()
        .map_err(|_| ApiError::bad_request(format!("limit 必须是 1 到 {max} 的整数")))?;
    if !(1..=max).contains(&limit) {
        return Err(ApiError::bad_request(format!(
            "limit 必须是 1 到 {max} 的整数"
        )));
    }
    Ok(limit)
}

fn optional_query_text(value: Option<String>, field: &str) -> Result<Option<String>, ApiError> {
    value
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(ApiError::bad_request(format!("{field} 不能为空")));
            }
            if trimmed.chars().count() > 200 {
                return Err(ApiError::bad_request(format!("{field} 过长")));
            }
            Ok(trimmed.to_string())
        })
        .transpose()
}

fn required_path_id<'a>(value: &'a str, field: &str) -> Result<&'a str, ApiError> {
    if value.is_empty() || value.trim() != value || value.chars().count() > 200 {
        return Err(ApiError::bad_request(format!("path {field} 无效")));
    }
    Ok(value)
}

fn require_matching_id(path: &str, body: &str, field: &str) -> Result<(), ApiError> {
    required_path_id(path, field)?;
    if path != body {
        return Err(ApiError::bad_request(format!(
            "body.{field} 必须与 path {field} 一致"
        )));
    }
    Ok(())
}

fn validate_event_batch(
    thread_id: &str,
    after_sequence: u64,
    events: &[AgentEvent],
) -> Result<(), ApiError> {
    let mut cursor = after_sequence;
    for event in events {
        if event.thread_id != thread_id {
            return Err(ApiError::conflict(
                "Agent event 回放返回了其他 Thread 的事件",
            ));
        }
        if event.sequence <= cursor {
            return Err(ApiError::conflict(
                "Agent event sequence 必须严格递增且大于游标",
            ));
        }
        cursor = event.sequence;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_api_contract::agent::AGENT_API_PROTOCOL_VERSION;
    use serde_json::json;

    fn event(thread_id: &str, sequence: u64) -> AgentEvent {
        AgentEvent {
            protocol_version: AGENT_API_PROTOCOL_VERSION,
            thread_id: thread_id.to_string(),
            sequence,
            kind: "message.accepted".to_string(),
            actor_kind: "user".to_string(),
            actor_id: Some("7".to_string()),
            turn_id: Some("11".to_string()),
            run_id: Some("12".to_string()),
            execution_id: None,
            delegation_id: None,
            message_id: Some("13".to_string()),
            payload: Some(json!({"public": true})),
            editor_request: None,
            created_at: "2026-08-27T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn event_cursor_uses_query_or_last_event_id_and_rejects_conflicts() {
        let mut headers = HeaderMap::new();
        assert_eq!(resolve_event_cursor(&headers, Some(4)).unwrap(), Some(4));

        headers.insert("last-event-id", HeaderValue::from_static("7"));
        assert_eq!(resolve_event_cursor(&headers, None).unwrap(), Some(7));
        assert_eq!(resolve_event_cursor(&headers, Some(7)).unwrap(), Some(7));
        assert!(resolve_event_cursor(&headers, Some(6)).is_err());

        headers.insert("last-event-id", HeaderValue::from_static("7:2"));
        assert!(resolve_event_cursor(&headers, None).is_err());
    }

    #[test]
    fn accepts_event_stream_in_a_media_range_list() {
        let mut headers = HeaderMap::new();
        assert!(!accepts_event_stream(&headers).unwrap());
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream; charset=utf-8"),
        );
        assert!(accepts_event_stream(&headers).unwrap());
    }

    #[test]
    fn limits_and_path_body_ids_are_strict() {
        assert_eq!(parse_limit(None, 50, 500).unwrap(), 50);
        assert_eq!(parse_limit(Some("500"), 50, 500).unwrap(), 500);
        assert!(parse_limit(Some("0"), 50, 500).is_err());
        assert!(parse_limit(Some("501"), 50, 500).is_err());
        assert!(parse_limit(Some(" 2"), 50, 500).is_err());

        require_matching_id("42", "42", "thread_id").unwrap();
        assert!(require_matching_id("42", "43", "thread_id").is_err());
        assert!(require_matching_id(" 42", " 42", "thread_id").is_err());
    }

    #[test]
    fn durable_event_batches_must_match_thread_and_increase_sequence() {
        validate_event_batch("9", 2, &[event("9", 3), event("9", 4)]).unwrap();
        assert!(validate_event_batch("9", 3, &[event("9", 3)]).is_err());
        assert!(validate_event_batch("9", 2, &[event("8", 3)]).is_err());
        assert!(validate_event_batch("9", 2, &[event("9", 4), event("9", 3)]).is_err());
    }

    #[test]
    fn sse_payload_uses_sequence_as_id_and_serializes_top_level_event() {
        let event = event("9", 3);
        let (id, data) = agent_event_payload(&event);
        assert_eq!(id, "3");
        let data: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(data["sequence"], 3);
        assert_eq!(data["thread_id"], "9");
        assert_eq!(data["kind"], "message.accepted");
    }

    #[test]
    fn agent_chat_payload_accepts_worker_snapshot() {
        let request = ChatRequest::system_user("system", "user", Some(0.2), Some(128))
            .with_json_response_format()
            .with_stream_usage();
        let value = request.snapshot_value().unwrap();
        let parsed: ChatRequest = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.temperature, Some(0.2));
        assert_eq!(parsed.max_tokens, Some(128));
        assert!(parsed.stream_include_usage);
        assert!(parsed.response_format.is_some());
    }
}
