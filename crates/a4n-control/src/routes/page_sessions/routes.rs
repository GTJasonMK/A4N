pub(super) async fn create(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreatePageSessionInput>,
) -> Result<(StatusCode, Json<PageSessionCreateResponse>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .page_sessions()
                .create(user.id, input, user.page_cli_enabled)?,
        ),
    ))
}

pub(super) async fn list(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PageSessionView>>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().list(user.id)))
}

pub(super) async fn get(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().get(user.id, &session_id)?))
}

pub(super) async fn authorize(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<PageAuthorization>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().authorize(
        user.id,
        &session_id,
        ui_token(&headers)?,
        user.page_cli_enabled,
    )?))
}

pub(super) async fn attach(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<AttachPageSessionInput>,
) -> Result<Json<PageAttachResponse>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().attach(
        user.id,
        &session_id,
        input,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
    )?))
}

pub(super) async fn attach_by_code(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AttachPageSessionInput>,
) -> Result<Json<PageAttachResponse>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().attach_by_code(
        user.id,
        input,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
    )?))
}

pub(super) async fn commands(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<PageCommandManifest>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().commands(
        user.id,
        &session_id,
        ui_token(&headers)?,
        user.role == "admin",
        user.page_cli_enabled,
    )?))
}

pub(super) async fn context(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageContext>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().update_context(
        user.id,
        &session_id,
        ui_token(&headers)?,
        input,
    )?))
}

pub(super) async fn heartbeat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageHeartbeatInput>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().heartbeat(
        user.id,
        &session_id,
        ui_token(&headers)?,
        input.context,
    )?))
}

pub(super) async fn start_operation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageOperationInput>,
) -> Result<(StatusCode, Json<PageOperationView>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok((
        StatusCode::CREATED,
        Json(state.page_sessions().start_operation(
            user.id,
            &session_id,
            ui_token(&headers)?,
            header_required(&headers, "x-agent-id")?.to_string(),
            state.agents(),
            agent_token(&headers)?,
            user.page_cli_enabled,
            user.role == "admin",
            input,
        )?),
    ))
}

pub(super) async fn update_operation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    path: Path<(String, String)>,
    Json(input): Json<PageOperationUpdate>,
) -> Result<Json<PageOperationView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let (session_id, operation_id) = path.0;
    Ok(Json(state.page_sessions().update_operation(
        user.id,
        &session_id,
        &operation_id,
        ui_token(&headers)?,
        header_required(&headers, "x-agent-id")?,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
        input,
    )?))
}

pub(super) async fn navigate(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageNavigationInput>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().navigate(
        user.id,
        &session_id,
        ui_token(&headers)?,
        header_required(&headers, "x-agent-id")?,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
        input,
    )?))
}

pub(super) async fn control(
    State(state): State<ServerState>,
    headers: HeaderMap,
    path: Path<(String, String)>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let (session_id, action) = path.0;
    Ok(Json(state.page_sessions().control(
        user.id,
        &session_id,
        ui_token(&headers)?,
        &action,
    )?))
}

pub(super) async fn events(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let stream = page_sse_stream(state.page_sessions().clone(), user.id);
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive"),
    ))
}
