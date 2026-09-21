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
    let user = authenticated_user(&state, &headers).await?;
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
    agent_actions::search_tools(state.runtime(), user.id, input)
        .await
        .map(Json)
        .map_err(Into::into)
}

pub(super) async fn describe_tool(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<AgentToolDescriptor>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let name = required_path_id(&name, "name")?.to_string();
    agent_actions::describe_tool(state.runtime(), user.id, DescribeAgentToolInput { name })
        .await
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
