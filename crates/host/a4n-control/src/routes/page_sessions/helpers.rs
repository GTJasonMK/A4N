fn page_sse_stream(
    registry: PageSessionRegistry,
    owner_user_id: i64,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    let receiver = registry.subscribe();
    let interval = tokio::time::interval(Duration::from_secs(5));
    stream::unfold(
        (registry, receiver, interval),
        move |(registry, mut receiver, mut interval)| async move {
            loop {
                tokio::select! {
                    result = receiver.recv() => match result {
                        Ok(event) if event.owner_user_id == owner_user_id => {
                            let data = serde_json::to_string(&event.event).expect("page event is serializable");
                            return Some((Ok(Event::default().event("page_session").data(data)), (registry, receiver, interval)));
                        }
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                    },
                    _ = interval.tick() => registry.expire_periodically(),
                }
            }
        },
    )
}

fn apply_context(view: &mut PageSessionView, context: PageContext) -> Result<(), ApiError> {
    if context.context_revision < view.context_revision {
        return Err(ApiError::conflict("Page Context revision 已过期"));
    }
    if let Some(route) = context.route.as_deref() {
        validate_route(route)?;
    }
    validate_positive_id(context.project_id, "project_id")?;
    validate_positive_id(context.chapter_id, "chapter_id")?;
    validate_json_size(&context.selected_ids, "selected_ids")?;
    validate_json_size(&context.local_state, "local_state")?;
    view.page_kind = context.page_kind;
    view.route = context.route;
    view.project_id = context.project_id;
    view.chapter_id = context.chapter_id;
    view.active_tab = context.active_tab;
    view.selected_ids = context.selected_ids;
    view.dirty_state = context.dirty_state;
    view.local_state = context.local_state;
    view.context_revision = context.context_revision;
    Ok(())
}

fn page_scope_matches(scope: &str, page_kind: Option<&str>) -> bool {
    if matches!(scope, "system" | "task" | "workflow") {
        return true;
    }
    let Some(page_kind) = page_kind else {
        return false;
    };
    let page_kind = page_kind.to_ascii_lowercase();
    page_kind.contains(scope)
        || match scope {
            "project" => page_kind.contains("novel") || page_kind.contains("workspace"),
            "auth" => page_kind.contains("setting") || page_kind.contains("account"),
            "chapter" | "outline" | "blueprint" | "profile" => {
                page_kind.contains("novel") || page_kind.contains("write")
            }
            "asset" | "video" => page_kind.contains("video") || page_kind.contains("media"),
            "settings" => page_kind.contains("setting"),
            _ => false,
        }
}

fn missing_page_context(fields: &[String], view: &PageSessionView) -> Option<String> {
    let missing = fields
        .iter()
        .filter(|field| match field.as_str() {
            "projectId" => view.project_id.is_none(),
            "chapterId" => view.chapter_id.is_none(),
            _ => true,
        })
        .cloned()
        .collect::<Vec<_>>();
    (!missing.is_empty()).then(|| format!("缺少当前页面上下文: {}", missing.join(", ")))
}

fn validate_page_command(
    command: &str,
    view: &PageSessionView,
    is_admin: bool,
    allow_destructive: bool,
) -> Result<(), ApiError> {
    let descriptor = command_manifest(None, None, None)
        .commands
        .into_iter()
        .find(|descriptor| descriptor.name == command)
        .ok_or_else(|| ApiError::bad_request(format!("当前页面未暴露 Host API 命令: {command}")))?;
    if !descriptor
        .page_scopes
        .iter()
        .any(|scope| page_scope_matches(scope, view.page_kind.as_deref()))
    {
        return Err(ApiError::conflict("当前页面未暴露该 Host API 命令"));
    }
    if descriptor.auth == "admin" && !is_admin {
        return Err(ApiError::forbidden("需要管理员权限"));
    }
    if let Some(reason) = missing_page_context(&descriptor.required_ui_context, view) {
        return Err(ApiError::conflict(reason));
    }
    if descriptor.effect == "destructive" && !allow_destructive {
        return Err(ApiError::bad_request(
            "危险 Page 命令需要 allow_destructive=true",
        ));
    }
    Ok(())
}

fn apply_operation_update(
    operation: &mut PageOperationView,
    update: PageOperationUpdate,
) -> Result<(), ApiError> {
    if is_terminal_operation_status(&operation.status)
        && (update.phase.is_some()
            || update.progress.is_some()
            || update.summary.is_some()
            || update.result.is_some()
            || update.error.is_some()
            || update
                .status
                .as_ref()
                .is_some_and(|status| status != &operation.status))
    {
        return Err(ApiError::conflict(
            "Page Operation 已进入终态，不能继续更新",
        ));
    }
    if let Some(progress) = update.progress {
        if !progress.is_finite() || !(0.0..=100.0).contains(&progress) {
            return Err(ApiError::bad_request("progress 必须在 0 到 100 之间"));
        }
        operation.progress = Some(progress);
    }
    if let Some(status) = update.status {
        if !operation_status_transition_allowed(&operation.status, &status) {
            return Err(ApiError::conflict(
                "Page Operation 已进入终态，不能再次改变状态",
            ));
        }
        operation.status = status;
    }
    if let Some(phase) = update.phase {
        operation.phase = required_text(phase, "phase")?;
    }
    if let Some(summary) = update.summary {
        operation.summary = Some(required_text(summary, "summary")?);
    }
    if let Some(error) = update.error {
        operation.error = Some(required_text(error, "error")?);
    }
    if let Some(result) = update.result {
        validate_json_size(&result, "result")?;
        operation.result = Some(result);
    }
    Ok(())
}

fn is_terminal_operation_status(status: &PageOperationStatus) -> bool {
    matches!(
        status,
        PageOperationStatus::Succeeded
            | PageOperationStatus::Failed
            | PageOperationStatus::Cancelled
    )
}

fn operation_status_transition_allowed(
    current: &PageOperationStatus,
    next: &PageOperationStatus,
) -> bool {
    match current {
        PageOperationStatus::Running => matches!(
            next,
            PageOperationStatus::Running
                | PageOperationStatus::Succeeded
                | PageOperationStatus::Failed
                | PageOperationStatus::Cancelled
        ),
        PageOperationStatus::Succeeded
        | PageOperationStatus::Failed
        | PageOperationStatus::Cancelled => current == next,
    }
}

fn push_operation(record: &mut PageRecord, operation: PageOperationView) {
    record.operations.push_back(operation.clone());
    while record.operations.len() > MAX_OPERATIONS {
        record.operations.pop_front();
    }
    record.view.last_operation = Some(operation);
}

fn cancel_operation(
    record: &mut PageRecord,
    operation_id: Option<&str>,
    reason: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<PageOperationView> {
    let operation = record
        .operations
        .iter_mut()
        .find(|operation| Some(operation.operation_id.as_str()) == operation_id)?;
    if !matches!(
        operation.status,
        PageOperationStatus::Succeeded
            | PageOperationStatus::Failed
            | PageOperationStatus::Cancelled
    ) {
        operation.status = PageOperationStatus::Cancelled;
        operation.error = Some(reason.to_string());
        operation.updated_at = now.to_rfc3339();
    }
    Some(operation.clone())
}

fn expire_record(
    record: &mut PageRecord,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<(i64, PageSessionView, Option<PageOperationView>)> {
    if record.view.expires_at > now.to_rfc3339()
        || record.view.status == PageSessionStatus::Offline
    {
        return None;
    }
    let operation_id = record.view.operation_id.clone();
    record.view.status = PageSessionStatus::Offline;
    record.view.agent_id = None;
    record.view.operation_id = None;
    let operation = cancel_operation(
        record,
        operation_id.as_deref(),
        "Page Session 心跳已过期",
        now.clone(),
    );
    record.view.last_operation = operation.clone();
    touch(&mut record.view, now);
    Some((record.owner_user_id, record.view.clone(), operation))
}

fn validate_operation_input(input: &PageOperationInput) -> Result<(), ApiError> {
    required_text(input.command.clone(), "command")?;
    validate_json_size(&input.target, "target")
}

fn validate_route(route: &str) -> Result<(), ApiError> {
    let route = required_text(route.to_string(), "route")?;
    if route.len() > MAX_ROUTE_LENGTH || !route.starts_with('/') || route.starts_with("//") {
        return Err(ApiError::bad_request("route 必须是站内绝对路径"));
    }
    Ok(())
}

fn validate_json_size(value: &Value, field: &str) -> Result<(), ApiError> {
    let size = serde_json::to_vec(value)
        .map_err(|_| ApiError::bad_request(format!("{field} 无效")))?
        .len();
    if size > MAX_STATE_BYTES {
        return Err(ApiError::bad_request(format!("{field} 不能超过 64 KiB")));
    }
    Ok(())
}

fn authorize_token(record: &PageRecord, owner_user_id: i64, token: &str) -> Result<(), ApiError> {
    if record.owner_user_id != owner_user_id || record.token != token {
        return Err(ApiError::unauthorized("Page Session token 无效"));
    }
    Ok(())
}

fn touch(view: &mut PageSessionView, now: chrono::DateTime<chrono::Utc>) {
    view.updated_at = now.to_rfc3339();
    view.last_seen_at = view.updated_at.clone();
    view.expires_at = lease_expiry(now);
}

fn lease_expiry(now: chrono::DateTime<chrono::Utc>) -> String {
    (now + chrono::Duration::seconds(PAGE_LEASE_SECONDS as i64)).to_rfc3339()
}

fn required_text(value: String, field: &str) -> Result<String, ApiError> {
    let value = value.trim().to_string();
    if value.is_empty() || value.chars().count() > 200 {
        return Err(ApiError::bad_request(format!("{field} 无效")));
    }
    Ok(value)
}

fn random_code_part() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(4)
        .collect::<String>()
        .to_uppercase()
}

fn ui_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    header_required(headers, "x-ui-session-token")
}

fn agent_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    header_required(headers, "x-agent-token")
}

fn header_required<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, ApiError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthorized(format!("缺少 {} 头", name)))
}
