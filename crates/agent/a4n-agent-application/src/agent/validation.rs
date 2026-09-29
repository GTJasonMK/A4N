//! Agent 输入、权限与 API projection helper。

use super::*;

pub(super) fn validate_user_id(user_id: i64) -> Result<(), AppError> {
    if user_id > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument("user_id 无效".into()))
    }
}

pub(super) fn validate_waiting_reason(reason: &str) -> Result<(), AppError> {
    match reason {
        "user" | "approval" | "child" | "external" | "backoff" | "budget" | "reconcile" => Ok(()),
        _ => Err(AppError::InvalidArgument(format!(
            "未知 Agent waiting reason: {reason}"
        ))),
    }
}

pub(super) fn validate_optional_positive_id(
    value: Option<i64>,
    field: &str,
) -> Result<(), AppError> {
    if value.is_some_and(|id| id <= 0) {
        Err(AppError::InvalidArgument(format!("{field} 无效")))
    } else {
        Ok(())
    }
}

pub(super) fn admit_new_run_runtime_snapshot(
    runtime_snapshot_required: bool,
    require_runtime_snapshot_for_new_run: bool,
) -> Result<(), AppError> {
    if require_runtime_snapshot_for_new_run && !runtime_snapshot_required {
        return Err(AppError::InvalidArgument(
            "创建 Agent run 必须提供 runtime_config_snapshot".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_model_override_json(value: Option<&Value>) -> Result<(), AppError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object().ok_or_else(|| AppError::InvalidState {
        current: "Agent turn model_override_json 必须是 object".into(),
        operation: "加载 Agent 模型配置".into(),
    })?;
    if object.len() != 1 {
        return Err(AppError::InvalidState {
            current: "Agent turn model_override_json 只能包含 model_config_id".into(),
            operation: "加载 Agent 模型配置".into(),
        });
    }
    object
        .get("model_config_id")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidState {
            current: "Agent turn model_override_json.model_config_id 无效".into(),
            operation: "加载 Agent 模型配置".into(),
        })?;
    Ok(())
}

pub(super) fn validate_editor_source(
    source: &a4n_api_contract::agent::AgentEditorSource,
    capability: &str,
    scope: &AgentScope,
) -> Result<(), AppError> {
    if source.protocol_version != 2 {
        return Err(AppError::InvalidArgument(
            "编辑器协议不兼容，请重新打开编辑器并发起新请求；旧请求不会自动重放".into(),
        ));
    }
    if capability != a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY
        || scope.project_id != Some(source.project_id)
        || scope.chapter_id != Some(source.chapter_id)
    {
        return Err(AppError::Forbidden(
            "编辑器来源必须属于当前导演 Agent 绑定的项目和章节".into(),
        ));
    }
    if source.chapter_number.is_some_and(|number| number <= 0) {
        return Err(AppError::InvalidArgument(
            "编辑器来源 chapter_number 无效".into(),
        ));
    }
    for value in [
        Some(&source.view_instance_id),
        Some(&source.editor_revision),
        source.scene_id.as_ref(),
        source.shot_id.as_ref(),
        source.keyframe_id.as_ref(),
        source.entity_id.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(AppError::InvalidArgument("编辑器来源标识无效".into()));
        }
    }
    Ok(())
}

pub(super) fn agent_scopes_match_except_chapter_number(
    left: &AgentScope,
    right: &AgentScope,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.chapter_number = None;
    right.chapter_number = None;
    left == right
}

pub(super) fn complete_scope_resource(
    scope: &mut AgentScope,
    kind: &str,
    id: &str,
) -> Result<(), AppError> {
    if (scope.resource_kind.is_some() || scope.resource_id.is_some())
        && (scope.resource_kind.as_deref() != Some(kind)
            || scope.resource_id.as_deref() != Some(id))
    {
        return Err(AppError::InvalidArgument(
            "Agent scope 与 bound_resource 不一致".into(),
        ));
    }
    scope.resource_kind = Some(kind.into());
    scope.resource_id = Some(id.into());
    Ok(())
}

pub(super) async fn canonicalize_editor_source(
    database: &sea_orm::DatabaseConnection,
    source: a4n_api_contract::agent::AgentEditorSource,
    thread: &thread::Model,
) -> Result<a4n_api_contract::agent::AgentEditorSource, AppError> {
    let scope: AgentScope = from_json(thread.scope_json.clone(), "agent_thread.scope_json")?;
    validate_editor_source(&source, &thread.profile_id, &scope)?;
    let chapter_number = a4n_db::project::resolve_video_chapter_source(
        database,
        source.project_id,
        source.chapter_id,
    )
    .await
    .map_err(agent_source_error)?
    .chapter_number;
    Ok(a4n_api_contract::agent::AgentEditorSource {
        chapter_number: Some(chapter_number),
        ..source
    })
}

// Binding a derived source permits context access, not mutation of the original
// chapter. Chapter-only command authorization retains its ownership check.
pub(super) async fn scope_chapter_number(
    database: &sea_orm::DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<i32, AppError> {
    let chapter = a4n_db::project::chapter_by_id(database, chapter_id).await?;
    if let Some(chapter) = chapter.filter(|chapter| chapter.project_id == project_id) {
        return Ok(chapter.chapter_number);
    }
    a4n_db::project::resolve_chapter_source(database, project_id, chapter_id)
        .await
        .map(|source| source.chapter_number)
        .map_err(agent_source_error)
}

pub(super) async fn verify_director_source(
    database: &sea_orm::DatabaseConnection,
    capability: &str,
    scope: &AgentScope,
) -> Result<(), AppError> {
    if capability == a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY {
        let project_id = scope
            .project_id
            .ok_or_else(|| AppError::InvalidArgument("导演 Agent 缺少目标视频项目".into()))?;
        let chapter_id = scope
            .chapter_id
            .ok_or_else(|| AppError::InvalidArgument("导演 Agent 缺少来源章节".into()))?;
        a4n_db::project::resolve_video_chapter_source(database, project_id, chapter_id)
            .await
            .map_err(agent_source_error)?;
    }
    Ok(())
}

fn agent_source_error(error: a4n_db::project::ResolveChapterSourceError) -> AppError {
    use a4n_db::project::ResolveChapterSourceError;
    match error {
        ResolveChapterSourceError::Database(error) => AppError::Database(error),
        ResolveChapterSourceError::NotFound(message) => AppError::NotFound(message),
        ResolveChapterSourceError::InvalidArgument(message) => AppError::Forbidden(message),
        ResolveChapterSourceError::InvalidState(current) => AppError::InvalidState {
            current,
            operation: "解析 Agent 章节来源".into(),
        },
    }
}

pub(super) async fn parse_persisted_editor_source(
    database: &sea_orm::DatabaseConnection,
    value: Value,
    thread: &thread::Model,
) -> Result<a4n_api_contract::agent::AgentEditorSource, AppError> {
    let scope: AgentScope = from_json(thread.scope_json.clone(), "agent_thread.scope_json")?;
    let value = normalize_legacy_editor_source(value, scope.chapter_id)?;
    let source = from_json(value, "agent_message.editor_source")?;
    canonicalize_editor_source(database, source, thread).await
}

pub(super) fn normalize_legacy_editor_source(
    mut value: Value,
    fallback_chapter_id: Option<i64>,
) -> Result<Value, AppError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "agent_message.editor_source 必须是 object".into(),
            operation: "恢复历史 Agent 编辑器来源".into(),
        })?;
    if !object.contains_key("chapter_id") {
        let chapter_id = fallback_chapter_id
            .filter(|chapter_id| *chapter_id > 0)
            .ok_or_else(|| AppError::InvalidState {
                current:
                    "历史 agent_message.editor_source 缺少 chapter_id，且线程 scope 未绑定章节"
                        .into(),
                operation: "恢复历史 Agent 编辑器来源".into(),
            })?;
        object.insert("chapter_id".into(), json!(chapter_id));
    }
    Ok(value)
}

pub(super) fn parse_id(value: &str, field: &str) -> Result<i64, AppError> {
    value
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 无效")))
}

pub(super) fn required_text(value: String, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{field} 不能为空")))
    } else {
        Ok(value.to_string())
    }
}

pub(super) fn optional_text(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>, AppError> {
    value.map(|value| required_text(value, field)).transpose()
}

pub(super) fn validate_scope(scope: &AgentScope) -> Result<(), AppError> {
    if scope.project_id.is_some_and(|id| id <= 0) || scope.chapter_id.is_some_and(|id| id <= 0) {
        return Err(AppError::InvalidArgument("Agent scope 中的 id 无效".into()));
    }
    if scope.chapter_number.is_some_and(|number| number <= 0) {
        return Err(AppError::InvalidArgument(
            "Agent scope 中的 chapter_number 无效".into(),
        ));
    }
    if scope.resource_kind.is_some() != scope.resource_id.is_some() {
        return Err(AppError::InvalidArgument(
            "Agent scope 的 resource_kind/resource_id 必须同时提供".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_resource_ref(resource: &AgentResourceRef) -> Result<(), AppError> {
    if resource.kind.trim().is_empty() || resource.id.trim().is_empty() {
        Err(AppError::InvalidArgument(
            "bound_resource.kind/id 不能为空".into(),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn required_capability(value: String) -> Result<String, AppError> {
    let value = required_text(value, "capability")?;
    if value.len() > 128 || value.chars().any(|ch| ch.is_control()) {
        return Err(AppError::InvalidArgument(
            "capability 长度或字符无效".into(),
        ));
    }
    Ok(value)
}

pub(super) fn inferred_subagent_resource(
    capability: &str,
    scope: &AgentScope,
) -> Result<Option<AgentResourceRef>, AppError> {
    let resource = match capability {
        a4n_api_contract::agent::CONTENT_OPTIMIZATION_AGENT_CAPABILITY => {
            let chapter_id = scope.chapter_id.ok_or_else(|| {
                AppError::InvalidArgument("content_optimization child 必须绑定 chapterId".into())
            })?;
            Some(AgentResourceRef {
                kind: "chapter".into(),
                id: chapter_id.to_string(),
                label: None,
                revision: None,
                hash: None,
            })
        }
        a4n_api_contract::agent::CANVAS_AGENT_CAPABILITY => {
            let project_id = scope.project_id.ok_or_else(|| {
                AppError::InvalidArgument("canvas child 必须绑定 projectId".into())
            })?;
            Some(AgentResourceRef {
                kind: "project".into(),
                id: project_id.to_string(),
                label: None,
                revision: None,
                hash: None,
            })
        }
        a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY => {
            let chapter_id = scope.chapter_id.ok_or_else(|| {
                AppError::InvalidArgument("director child 必须绑定 chapterId".into())
            })?;
            Some(AgentResourceRef {
                kind: "chapter".into(),
                id: chapter_id.to_string(),
                label: None,
                revision: None,
                hash: None,
            })
        }
        _ => match (scope.resource_kind.as_deref(), scope.resource_id.as_deref()) {
            (Some(kind), Some(id)) => Some(AgentResourceRef {
                kind: kind.to_string(),
                id: id.to_string(),
                label: None,
                revision: None,
                hash: None,
            }),
            (None, None) if scope.chapter_id.is_some() => Some(AgentResourceRef {
                kind: "chapter".into(),
                id: scope.chapter_id.expect("checked Some").to_string(),
                label: None,
                revision: None,
                hash: None,
            }),
            (None, None) if scope.project_id.is_some() => Some(AgentResourceRef {
                kind: "project".into(),
                id: scope.project_id.expect("checked Some").to_string(),
                label: None,
                revision: None,
                hash: None,
            }),
            (None, None) => None,
            _ => {
                return Err(AppError::InvalidArgument(
                    "Agent scope 的 resource_kind/resource_id 必须同时提供".into(),
                ));
            }
        },
    };
    Ok(resource)
}

pub(super) fn validate_subagent_resource(
    capability: &str,
    scope: &AgentScope,
    resource: Option<&AgentResourceRef>,
) -> Result<(), AppError> {
    match capability {
        a4n_api_contract::agent::CONTENT_OPTIMIZATION_AGENT_CAPABILITY
            if resource.is_none_or(|resource| resource.kind != "chapter")
                || scope.project_id.is_none()
                || scope.chapter_id.is_none() =>
        {
            return Err(AppError::InvalidArgument(
                "content_optimization child 必须绑定 project/chapter".into(),
            ));
        }
        a4n_api_contract::agent::CANVAS_AGENT_CAPABILITY
            if resource.is_none_or(|resource| resource.kind != "project")
                || scope.project_id.is_none()
                || scope.chapter_id.is_some() =>
        {
            return Err(AppError::InvalidArgument(
                "canvas child 只能绑定 project，不能绑定 chapter".into(),
            ));
        }
        a4n_api_contract::agent::DIRECTOR_AGENT_CAPABILITY
            if resource.is_none_or(|resource| resource.kind != "chapter")
                || scope.project_id.is_none()
                || scope.chapter_id.is_none() =>
        {
            return Err(AppError::InvalidArgument(
                "director child 必须绑定 project/chapter".into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

/// Apply the one tool-scope rule shared by direct thread creation and
/// parent-driven delegation. Empty means the capability's current catalog;
/// only the worker materializes that catalog, so stored defaults do not go
/// stale. Existing nonempty lists remain explicit restrictions.
pub(super) fn apply_subagent_tool_policy(
    capability: &str,
    scope: &mut AgentScope,
) -> Result<(), AppError> {
    if a4n_api_contract::agent::capability_has_restricted_tool_catalog(capability) {
        let allowed_catalog = a4n_api_contract::agent::agent_tools_for_capability(capability);
        for tool in &scope.allowed_tools {
            if !allowed_catalog.contains(&tool.as_str()) {
                return Err(AppError::Forbidden(format!(
                    "capability {capability} 不允许使用工具: {tool}"
                )));
            }
        }
        return Ok(());
    }

    for tool in &scope.allowed_tools {
        let descriptor = a4n_api_contract::agent::agent_tool_descriptor(tool)
            .ok_or_else(|| AppError::Forbidden(format!("Agent 工具不存在或不可发现: {tool}")))?;
        if descriptor
            .agent_policy
            .as_ref()
            .is_none_or(|policy| policy.exposure == AgentToolExposure::Hidden)
        {
            return Err(AppError::Forbidden(format!(
                "Agent 工具不允许暴露给通用子 Agent: {tool}"
            )));
        }
    }
    Ok(())
}

pub(super) fn effective_agent_scope(
    capability: &str,
    mut scope: AgentScope,
) -> Result<AgentScope, AppError> {
    apply_subagent_tool_policy(capability, &mut scope)?;
    if scope.allowed_tools.is_empty()
        && a4n_api_contract::agent::capability_has_restricted_tool_catalog(capability)
    {
        scope.allowed_tools = a4n_api_contract::agent::agent_tools_for_capability(capability)
            .iter()
            .map(|tool| (*tool).to_string())
            .collect();
    }
    Ok(scope)
}

/// Capability allowlists describe which profiles a run may delegate to. An
/// empty list is intentionally open; registered domain children are the only
/// profiles forced into a single-capability boundary.
pub(super) fn apply_subagent_capability_policy(
    capability: &str,
    scope: &mut AgentScope,
) -> Result<(), AppError> {
    if a4n_api_contract::agent::capability_has_restricted_tool_catalog(capability) {
        if scope.allowed_capabilities.is_empty() {
            scope.allowed_capabilities = vec![capability.to_string()];
        } else if scope
            .allowed_capabilities
            .iter()
            .any(|allowed| allowed != capability)
        {
            return Err(AppError::Forbidden(format!(
                "capability {capability} 只能委派同一 capability"
            )));
        } else {
            scope.allowed_capabilities = vec![capability.to_string()];
        }
    }
    Ok(())
}

/// Child runs must not widen a parent's explicit capability/tool allowlists.
/// Empty means unrestricted and therefore inherits no additional fence.
pub fn inherit_subagent_allowlists(
    parent: &AgentScope,
    child: &mut AgentScope,
) -> Result<(), AppError> {
    if !parent.allowed_capabilities.is_empty() {
        if child.allowed_capabilities.is_empty() {
            child.allowed_capabilities = parent.allowed_capabilities.clone();
        } else if child.allowed_capabilities.iter().any(|allowed| {
            !parent
                .allowed_capabilities
                .iter()
                .any(|item| item == allowed)
        }) {
            return Err(AppError::Forbidden(
                "child Agent capability allowlist 不能超出 parent Agent".into(),
            ));
        }
    }
    if !parent.allowed_tools.is_empty() {
        if child.allowed_tools.is_empty() {
            child.allowed_tools = parent.allowed_tools.clone();
        } else if child
            .allowed_tools
            .iter()
            .any(|allowed| !parent.allowed_tools.iter().any(|item| item == allowed))
        {
            return Err(AppError::Forbidden(
                "child Agent tool allowlist 不能超出 parent Agent".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn canonical_subagent_binding_key(
    capability: &str,
    project_id: Option<i64>,
    resource: &AgentResourceRef,
) -> Result<String, AppError> {
    let capability = required_text(capability.to_string(), "capability")?;
    let kind = required_text(resource.kind.clone(), "bound_resource.kind")?;
    let id = required_text(resource.id.clone(), "bound_resource.id")?;
    if kind == "chapter" && project_id.is_none() {
        return Err(AppError::InvalidArgument(
            "章节绑定必须声明 project_id".into(),
        ));
    }
    Ok(a4n_api_contract::agent::canonical_subagent_binding_key(
        &capability,
        project_id,
        &kind,
        &id,
    ))
}

pub(super) fn collect_scoped_resource(
    kind: &str,
    id: &str,
    project_id: &mut Option<i64>,
    chapter_id: &mut Option<i64>,
) -> Result<(), AppError> {
    match kind.trim() {
        "project" => merge_scope_id(
            project_id,
            parse_id(id, "project resource id")?,
            "project_id",
        ),
        "chapter" => merge_scope_id(
            chapter_id,
            parse_id(id, "chapter resource id")?,
            "chapter_id",
        ),
        _ => Ok(()),
    }
}

pub(super) fn merge_scope_id(
    target: &mut Option<i64>,
    value: i64,
    field: &str,
) -> Result<(), AppError> {
    match *target {
        Some(existing) if existing != value => Err(AppError::InvalidArgument(format!(
            "Agent scope 中的 {field} 相互冲突: {existing} != {value}"
        ))),
        Some(_) => Ok(()),
        None => {
            *target = Some(value);
            Ok(())
        }
    }
}

pub(super) fn bounded_limit(
    value: Option<u32>,
    default: u32,
    max: u32,
    field: &str,
) -> Result<u32, AppError> {
    let value = value.unwrap_or(default);
    if value == 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value.min(max))
}

pub(super) fn normalized_capability(
    kind: AgentThreadKind,
    capability: Option<String>,
) -> Result<String, AppError> {
    match kind {
        AgentThreadKind::Main => match capability {
            Some(value) if value.trim() != MAIN_AGENT_CAPABILITY => Err(AppError::InvalidArgument(
                "主 Agent capability 必须为 main".into(),
            )),
            _ => Ok(MAIN_AGENT_CAPABILITY.into()),
        },
        AgentThreadKind::Subagent => required_capability(
            capability
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(GENERAL_AGENT_CAPABILITY)
                .to_string(),
        ),
    }
}

pub(super) fn enum_value<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .expect("agent enums serialize")
        .as_str()
        .expect("agent enums serialize to strings")
        .to_string()
}

pub(super) fn delivery_inbox_kind(delivery: AgentDeliveryMode) -> AgentInboxKind {
    match delivery {
        AgentDeliveryMode::Followup => AgentInboxKind::Followup,
        AgentDeliveryMode::Steer => AgentInboxKind::Steer,
        AgentDeliveryMode::Inject => AgentInboxKind::Inject,
    }
}

pub(super) fn delivery_priority(delivery: AgentDeliveryMode) -> i32 {
    match delivery {
        AgentDeliveryMode::Followup => 0,
        AgentDeliveryMode::Steer => 10,
        AgentDeliveryMode::Inject => 20,
    }
}

pub(super) fn validate_control_transition(
    status: &str,
    waiting_reason: Option<&str>,
    action: AgentControlAction,
    has_budget_update: bool,
) -> Result<(), AppError> {
    if has_budget_update && action != AgentControlAction::Resume {
        return Err(AppError::InvalidArgument(
            "只能在恢复 Agent run 时调整预算".into(),
        ));
    }
    let allowed = match action {
        AgentControlAction::Pause => matches!(status, "queued" | "running" | "waiting"),
        AgentControlAction::Resume => {
            matches!(status, "paused" | "pausing")
                || (status == "waiting" && waiting_reason == Some("budget"))
        }
        AgentControlAction::Cancel => !matches!(
            status,
            "completed" | "failed" | "cancelled" | "interrupted" | "recovery_required"
        ),
    };
    if allowed {
        Ok(())
    } else {
        Err(AppError::InvalidState {
            current: status.to_string(),
            operation: format!("{:?} Agent run", action).to_lowercase(),
        })
    }
}

#[cfg(test)]
mod control_transition_tests {
    use super::validate_control_transition;
    use a4n_api_contract::agent::AgentControlAction;

    #[test]
    fn budget_wait_can_resume_without_bypassing_the_existing_budget() {
        assert!(
            validate_control_transition(
                "waiting",
                Some("budget"),
                AgentControlAction::Resume,
                false,
            )
            .is_ok()
        );
        assert!(
            validate_control_transition(
                "paused",
                Some("budget"),
                AgentControlAction::Resume,
                false,
            )
            .is_ok()
        );
        assert!(
            validate_control_transition(
                "waiting",
                Some("budget"),
                AgentControlAction::Resume,
                true,
            )
            .is_ok()
        );
        assert!(
            validate_control_transition(
                "waiting",
                Some("approval"),
                AgentControlAction::Resume,
                true,
            )
            .is_err()
        );
    }
}

pub(super) fn accepted(
    thread_id: i64,
    message_id: i64,
    turn_id: i64,
    run_id: i64,
    status: &str,
    created_run: bool,
) -> Result<AgentMessageAccepted, AppError> {
    Ok(AgentMessageAccepted {
        thread_id: thread_id.to_string(),
        message_id: message_id.to_string(),
        turn_id: turn_id.to_string(),
        run_id: run_id.to_string(),
        status: parse_run_status(status)?,
        created_run,
    })
}

pub(super) fn thread_view(row: thread::Model) -> Result<AgentThreadView, AppError> {
    let scope = from_json(row.scope_json, "agent_thread.scope_json")?;
    let bound_resource = match (row.resource_type, row.resource_id) {
        (Some(kind), Some(id)) => Some(AgentResourceRef {
            kind,
            id,
            label: None,
            revision: None,
            hash: None,
        }),
        (None, None) => None,
        _ => {
            return Err(AppError::InvalidState {
                current: format!("Agent thread {} 的资源绑定不完整", row.id),
                operation: "读取 Agent thread".into(),
            });
        }
    };
    let context_summary = parse_agent_context_summary(row.context_summary.as_deref())?;
    Ok(AgentThreadView {
        protocol_version: AGENT_API_PROTOCOL_VERSION,
        id: row.id.to_string(),
        owner_user_id: row.owner_user_id,
        kind: parse_thread_kind(&row.thread_kind)?,
        capability: Some(row.profile_id),
        title: row.title,
        bound_resource,
        scope,
        default_model_config_id: row.default_model_config_id,
        status: if row.archived_at.is_some() || row.status == "archived" {
            AgentThreadStatus::Archived
        } else {
            AgentThreadStatus::Active
        },
        revision: non_negative_u64(row.revision, "agent_thread.revision")?,
        // Persistence stores the latest allocated sequence; the public contract exposes the next one.
        next_turn_sequence: next_sequence_view(row.next_turn_sequence, "next_turn_sequence")?,
        next_event_sequence: next_sequence_view(row.next_event_sequence, "next_event_sequence")?,
        context_summary,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

pub(super) fn next_sequence_view(value: i64, field: &str) -> Result<u64, AppError> {
    non_negative_u64(value, field)?
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("{field} 超出范围"),
            operation: "读取 Agent thread 序列".into(),
        })
}

pub(super) fn message_view(row: message::Model) -> Result<AgentMessageView, AppError> {
    Ok(AgentMessageView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        turn_id: row.turn_id.map(|id| id.to_string()),
        run_id: row.run_id.map(|id| id.to_string()),
        role: parse_message_role(&row.role)?,
        status: parse_message_status(&row.status)?,
        content: row.content,
        source_kind: Some(row.sender_kind),
        source_id: row.sender_thread_id.map(|id| id.to_string()),
        metadata: row.payload_json,
        created_at: row.created_at.to_string(),
        updated_at: Some(row.updated_at.to_string()),
    })
}

pub(super) fn turn_view(row: turn::Model) -> Result<AgentTurnView, AppError> {
    Ok(AgentTurnView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        sequence: non_negative_u64(row.sequence, "agent_turn.sequence")?,
        trigger: parse_turn_trigger(&row.trigger)?,
        goal: row.goal,
        status: parse_turn_status(&row.status)?,
        final_message_id: row.final_message_id.map(|id| id.to_string()),
        result_refs: optional_json_vec(row.result_refs_json, "agent_turn.result_refs_json")?,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

pub(super) fn run_view(row: run::Model) -> Result<AgentRunView, AppError> {
    runtime_snapshot_required(&row.usage_json, row.id)?;
    let usage = row
        .usage_json
        .as_object()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent run {} usage_json 不是对象", row.id),
            operation: "读取 Agent run".into(),
        })?;
    Ok(AgentRunView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        turn_id: row.turn_id.to_string(),
        attempt: non_negative_u32(row.attempt, "agent_run.attempt")?,
        status: parse_run_status(&row.status)?,
        waiting_reason: row
            .waiting_reason
            .as_deref()
            .map(parse_waiting_reason)
            .transpose()?,
        lease_owner: row.worker_id,
        lease_expires_at: row
            .lease_expires_at
            .map(|value| value.and_utc().to_rfc3339()),
        budget: from_json::<AgentBudget>(row.budget_json, "agent_run.budget_json")?
            .effective_for_run(row.parent_run_id.is_none()),
        steps_used: json_u32(usage, "steps_used")?,
        model_calls_used: json_u32(usage, "model_calls_used")?,
        tool_calls_used: json_u32(usage, "tool_calls_used")?,
        tokens_used: json_u64(usage, "tokens_used")?,
        tokens_reserved: json_u64(usage, "tokens_reserved")?,
        usage_complete: usage
            .get("usage_complete")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        budget_limiting_run_id: usage
            .get("budget_limiting_run_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        context_input_tokens: json_optional_u64(usage, "context_input_tokens")?,
        context_capacity_tokens: json_optional_u64(usage, "context_capacity_tokens")?,
        cost_micros: json_optional_u64(usage, "cost_micros")?,
        error: row.last_error,
        created_at: row.created_at.and_utc().to_rfc3339(),
        started_at: row.started_at.map(|value| value.and_utc().to_rfc3339()),
        updated_at: row.updated_at.and_utc().to_rfc3339(),
    })
}

pub(super) fn runtime_snapshot_required(usage_json: &Value, run_id: i64) -> Result<bool, AppError> {
    let usage = usage_json
        .as_object()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent run {run_id} usage_json 不是对象"),
            operation: "读取 Agent runtime snapshot admission".into(),
        })?;
    match usage.get("runtime_snapshot_required") {
        Some(Value::Bool(required)) => Ok(*required),
        Some(_) => Err(AppError::InvalidState {
            current: format!("Agent run {run_id} runtime_snapshot_required 不是 boolean"),
            operation: "读取 Agent runtime snapshot admission".into(),
        }),
        None => Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 缺少 runtime_snapshot_required 标记"),
            operation: "读取 Agent runtime snapshot admission".into(),
        }),
    }
}

pub(super) fn execution_view(row: execution::Model) -> Result<AgentExecutionView, AppError> {
    Ok(AgentExecutionView {
        id: row.id.to_string(),
        run_id: row.run_id.to_string(),
        sequence: non_negative_u64(row.sequence, "agent_execution.sequence")?,
        kind: parse_execution_kind(&row.execution_kind)?,
        tool_name: row.tool_name,
        status: parse_execution_status(&row.status)?,
        effect: row.effect,
        replay_policy: row
            .replay_policy
            .as_deref()
            .map(parse_replay_policy)
            .transpose()?,
        request_digest: row.arguments_sha256,
        resource_refs: resource_refs_from_value(
            row.resource_preconditions_json,
            "agent_execution.resource_preconditions_json",
        )?,
        result_refs: optional_json_vec(row.result_refs_json, "agent_execution.result_refs_json")?,
        result_summary: row.result_summary,
        error: row.error_message,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

pub(super) fn delegation_view(row: delegation::Model) -> Result<AgentDelegationView, AppError> {
    Ok(AgentDelegationView {
        id: row.id.to_string(),
        parent_run_id: row.parent_run_id.to_string(),
        parent_execution_id: row.parent_execution_id.map(|id| id.to_string()),
        child_thread_id: row.child_thread_id.to_string(),
        child_turn_id: row.child_turn_id.map(|id| id.to_string()),
        child_run_id: row.child_run_id.map(|id| id.to_string()),
        capability: row.capability,
        task: row.objective,
        scope: from_json(row.scope_json, "agent_delegation.scope_json")?,
        status: parse_delegation_status(&row.status)?,
        success_criteria: from_json(row.success_criteria_json, "success_criteria_json")?,
        result_refs: optional_json_vec(row.result_refs_json, "delegation.result_refs_json")?,
        acceptance_summary: row.result_summary,
        acceptance_error: row.error_message,
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

pub(super) fn approval_view(row: approval::Model) -> Result<AgentApprovalView, AppError> {
    let operation = row
        .operation_json
        .as_object()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent approval {} operation_json 不是对象", row.id),
            operation: "读取 Agent approval".into(),
        })?;
    let status = parse_approval_status(&row.status)?;
    Ok(AgentApprovalView {
        id: row.id.to_string(),
        execution_id: row.execution_id.to_string(),
        status,
        command: operation
            .get("command")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::InvalidState {
                current: format!("Agent approval {} 缺少 command", row.id),
                operation: "读取 Agent approval".into(),
            })?
            .to_string(),
        args_digest: operation
            .get("args_digest")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(Some(row.operation_digest)),
        resource_refs: operation
            .get("resource_refs")
            .cloned()
            .map(|value| from_json(value, "approval.resource_refs"))
            .transpose()?
            .unwrap_or_default(),
        resource_revision: operation
            .get("resource_revision")
            .and_then(Value::as_str)
            .map(str::to_string),
        expected_effect: operation
            .get("expected_effect")
            .and_then(Value::as_str)
            .map(str::to_string),
        expires_at: row.expires_at.map(|value| value.to_string()),
        decided_by_user_id: row.decided_by_user_id,
        decision: match status {
            AgentApprovalStatus::Approved => Some(AgentApprovalDecision::Approve),
            AgentApprovalStatus::Rejected => Some(AgentApprovalDecision::Reject),
            _ => None,
        },
        reason: row.decision_note,
        created_at: row.created_at.to_string(),
        decided_at: row.decided_at.map(|value| value.to_string()),
    })
}

pub(super) fn inbox_view(row: inbox::Model) -> Result<AgentInboxItemView, AppError> {
    Ok(AgentInboxItemView {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        kind: parse_inbox_kind(&row.inbox_kind)?,
        status: parse_inbox_status(&row.status)?,
        delivery_mode: Some(parse_delivery_mode(&row.delivery_mode)?),
        message_id: row
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("message_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        content: Some(row.content),
        payload: row.payload_json,
        priority: i16::try_from(row.priority).map_err(|_| AppError::InvalidState {
            current: format!("Agent inbox {} priority 超出 i16", row.id),
            operation: "读取 Agent inbox".into(),
        })?,
        created_at: row.created_at.to_string(),
        consumed_at: row.consumed_at.map(|value| value.to_string()),
    })
}

pub(super) fn memory_view(row: memory::Model) -> Result<AgentMemoryView, AppError> {
    let resource_refs = row
        .content_json
        .as_ref()
        .and_then(|value| value.get("resource_refs"))
        .cloned()
        .map(|value| from_json(value, "agent_memory.resource_refs"))
        .transpose()?
        .unwrap_or_default();
    Ok(AgentMemoryView {
        id: row.id.to_string(),
        scope_kind: parse_memory_scope_kind(&row.scope_kind)?,
        scope_id: row.scope_key,
        status: parse_memory_status(&row.status)?,
        memory_kind: row.memory_key,
        content: row.content,
        resource_refs,
        source_thread_id: row.thread_id.map(|id| id.to_string()),
        source_message_id: row.source_message_id.map(|id| id.to_string()),
        supersedes_id: row.supersedes_memory_id.map(|id| id.to_string()),
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    })
}

pub(super) fn event_view(row: event::Model) -> Result<AgentEvent, AppError> {
    let editor_request = row
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("editor_request"))
        .filter(|value| !value.is_null())
        .cloned()
        .map(|value| from_json(value, "agent_event.editor_request"))
        .transpose()?;
    Ok(AgentEvent {
        protocol_version: AGENT_API_PROTOCOL_VERSION,
        thread_id: row.thread_id.to_string(),
        sequence: non_negative_u64(row.sequence, "agent_event.sequence")?,
        kind: row.kind,
        actor_kind: row.actor_kind,
        actor_id: row.actor_id,
        turn_id: row.turn_id.map(|id| id.to_string()),
        run_id: row.run_id.map(|id| id.to_string()),
        execution_id: row.execution_id.map(|id| id.to_string()),
        delegation_id: row.delegation_id.map(|id| id.to_string()),
        message_id: row.message_id.map(|id| id.to_string()),
        payload: event_payload(row.payload_json, row.public_text, row.status),
        editor_request,
        created_at: row.created_at.to_string(),
    })
}

pub(super) fn event_payload(
    payload: Option<Value>,
    public_text: String,
    status: String,
) -> Option<Value> {
    match payload {
        Some(Value::Object(mut object)) => {
            if !public_text.is_empty() {
                object.entry("text").or_insert(Value::String(public_text));
            }
            object.entry("status").or_insert(Value::String(status));
            Some(Value::Object(object))
        }
        Some(payload) => Some(json!({
            "data": payload,
            "text": public_text,
            "status": status,
        })),
        None if public_text.is_empty() && status.is_empty() => None,
        None => Some(json!({ "text": public_text, "status": status })),
    }
}

pub(super) fn from_json<T: DeserializeOwned>(value: Value, field: &str) -> Result<T, AppError> {
    serde_json::from_value(value).map_err(|error| AppError::InvalidState {
        current: format!("{field} 数据损坏: {error}"),
        operation: "读取 Agent 持久化状态".into(),
    })
}

pub(super) fn optional_json_vec<T: DeserializeOwned>(
    value: Option<Value>,
    field: &str,
) -> Result<Vec<T>, AppError> {
    value
        .map(|value| from_json(value, field))
        .transpose()
        .map(Option::unwrap_or_default)
}

pub(super) fn resource_refs_from_value(
    value: Option<Value>,
    field: &str,
) -> Result<Vec<AgentResourceRef>, AppError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    match value {
        Value::Array(_) => from_json(value, field),
        Value::Object(mut object) => object
            .remove("resource_refs")
            .map(|value| from_json(value, field))
            .transpose()
            .map(Option::unwrap_or_default),
        _ => Err(AppError::InvalidState {
            current: format!("{field} 既不是数组也不是对象"),
            operation: "读取 Agent 持久化状态".into(),
        }),
    }
}

pub(super) fn non_negative_u64(value: i64, field: &str) -> Result<u64, AppError> {
    u64::try_from(value).map_err(|_| AppError::InvalidState {
        current: format!("{field} 为负数: {value}"),
        operation: "读取 Agent 持久化状态".into(),
    })
}

pub(super) fn non_negative_u32(value: i32, field: &str) -> Result<u32, AppError> {
    u32::try_from(value).map_err(|_| AppError::InvalidState {
        current: format!("{field} 为负数: {value}"),
        operation: "读取 Agent 持久化状态".into(),
    })
}

pub(super) fn json_u32(map: &serde_json::Map<String, Value>, key: &str) -> Result<u32, AppError> {
    u32::try_from(json_u64(map, key)?).map_err(|_| AppError::InvalidState {
        current: format!("Agent usage.{key} 超出 u32"),
        operation: "读取 Agent run".into(),
    })
}

pub(super) fn json_u64(map: &serde_json::Map<String, Value>, key: &str) -> Result<u64, AppError> {
    match map.get(key) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| AppError::InvalidState {
            current: format!("Agent usage.{key} 不是非负整数"),
            operation: "读取 Agent run".into(),
        }),
    }
}

pub(super) fn json_optional_u64(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<u64>, AppError> {
    map.get(key)
        .map(|value| {
            value.as_u64().ok_or_else(|| AppError::InvalidState {
                current: format!("Agent usage.{key} 不是非负整数"),
                operation: "读取 Agent run".into(),
            })
        })
        .transpose()
}

macro_rules! parse_enum {
    ($name:ident, $ty:ty, {$($($raw:literal)|+ => $value:path),+ $(,)?}) => {
        pub(super) fn $name(raw: &str) -> Result<$ty, AppError> {
            match raw {
                $($($raw)|+ => Ok($value),)+
                other => Err(AppError::InvalidState {
                    current: format!("未知 Agent 状态值: {other}"),
                    operation: "投影 Agent API DTO".into(),
                }),
            }
        }
    };
}

parse_enum!(parse_thread_kind, AgentThreadKind, {"main" => AgentThreadKind::Main, "subagent" => AgentThreadKind::Subagent});
parse_enum!(parse_message_role, AgentMessageRole, {"user" => AgentMessageRole::User, "assistant" => AgentMessageRole::Assistant, "system" => AgentMessageRole::System, "tool" => AgentMessageRole::Tool});
parse_enum!(parse_message_status, AgentMessageStatus, {"streaming" => AgentMessageStatus::Streaming, "complete" => AgentMessageStatus::Complete, "interrupted" => AgentMessageStatus::Interrupted});
parse_enum!(parse_turn_trigger, AgentTurnTrigger, {"user" => AgentTurnTrigger::User, "delegation" => AgentTurnTrigger::Delegation, "recovery" => AgentTurnTrigger::Recovery});
parse_enum!(parse_turn_status, AgentTurnStatus, {"queued" => AgentTurnStatus::Queued, "active" => AgentTurnStatus::Active, "completed" => AgentTurnStatus::Completed, "failed" => AgentTurnStatus::Failed, "cancelled" => AgentTurnStatus::Cancelled});
parse_enum!(parse_run_status, AgentRunStatus, {"queued" => AgentRunStatus::Queued, "running" => AgentRunStatus::Running, "waiting" => AgentRunStatus::Waiting, "pausing" => AgentRunStatus::Pausing, "paused" => AgentRunStatus::Paused, "cancelling" => AgentRunStatus::Cancelling, "cancelled" => AgentRunStatus::Cancelled, "completed" => AgentRunStatus::Completed, "failed" => AgentRunStatus::Failed, "interrupted" => AgentRunStatus::Interrupted, "recovery_required" => AgentRunStatus::RecoveryRequired});
parse_enum!(parse_waiting_reason, AgentWaitingReason, {"user" => AgentWaitingReason::User, "approval" => AgentWaitingReason::Approval, "child" => AgentWaitingReason::Child, "external" => AgentWaitingReason::External, "backoff" => AgentWaitingReason::Backoff, "budget" => AgentWaitingReason::Budget, "reconcile" => AgentWaitingReason::Reconcile});
parse_enum!(parse_execution_kind, AgentExecutionKind, {"model_call" => AgentExecutionKind::ModelCall, "tool_call" => AgentExecutionKind::ToolCall, "wait" => AgentExecutionKind::Wait, "delegation" => AgentExecutionKind::Delegation, "verification" => AgentExecutionKind::Verification});
parse_enum!(parse_execution_status, AgentExecutionStatus, {"prepared" => AgentExecutionStatus::Prepared, "awaiting_user" => AgentExecutionStatus::AwaitingUser, "dispatching" => AgentExecutionStatus::Dispatching, "running" => AgentExecutionStatus::Running, "succeeded" => AgentExecutionStatus::Succeeded, "failed" => AgentExecutionStatus::Failed, "rejected" => AgentExecutionStatus::Rejected, "cancelled" => AgentExecutionStatus::Cancelled, "outcome_unknown" => AgentExecutionStatus::OutcomeUnknown});
parse_enum!(parse_replay_policy, AgentToolReplayPolicy, {"safe" => AgentToolReplayPolicy::Safe, "idempotent" => AgentToolReplayPolicy::Idempotent, "reconcile_required" => AgentToolReplayPolicy::ReconcileRequired, "never" => AgentToolReplayPolicy::Never});
parse_enum!(parse_delegation_status, AgentDelegationStatus, {"pending" => AgentDelegationStatus::Pending, "running" => AgentDelegationStatus::Running, "awaiting_acceptance" => AgentDelegationStatus::AwaitingAcceptance, "revision_requested" => AgentDelegationStatus::RevisionRequested, "accepted" => AgentDelegationStatus::Accepted, "failed" => AgentDelegationStatus::Failed, "cancelled" => AgentDelegationStatus::Cancelled, "interrupted" => AgentDelegationStatus::Interrupted});
parse_enum!(parse_approval_status, AgentApprovalStatus, {"pending" => AgentApprovalStatus::Pending, "approved" => AgentApprovalStatus::Approved, "rejected" => AgentApprovalStatus::Rejected, "expired" => AgentApprovalStatus::Expired, "consumed" => AgentApprovalStatus::Consumed});
parse_enum!(parse_inbox_kind, AgentInboxKind, {"followup" => AgentInboxKind::Followup, "steer" => AgentInboxKind::Steer, "inject" => AgentInboxKind::Inject, "control" => AgentInboxKind::Control, "child_result" => AgentInboxKind::ChildResult});
parse_enum!(parse_inbox_status, AgentInboxStatus, {"pending" => AgentInboxStatus::Pending, "claimed" => AgentInboxStatus::Claimed, "consumed" => AgentInboxStatus::Consumed, "superseded" => AgentInboxStatus::Superseded, "cancelled" => AgentInboxStatus::Cancelled});
parse_enum!(parse_delivery_mode, AgentDeliveryMode, {"followup" => AgentDeliveryMode::Followup, "steer" => AgentDeliveryMode::Steer, "inject" => AgentDeliveryMode::Inject});
parse_enum!(parse_memory_scope_kind, AgentMemoryScopeKind, {"user" => AgentMemoryScopeKind::User, "project" => AgentMemoryScopeKind::Project, "thread" => AgentMemoryScopeKind::Thread});
parse_enum!(parse_memory_status, AgentMemoryStatus, {"proposed" => AgentMemoryStatus::Proposed, "active" => AgentMemoryStatus::Active, "superseded" => AgentMemoryStatus::Superseded, "retracted" => AgentMemoryStatus::Retracted});

pub(super) fn agent_db_error_to_app(error: AgentDbError) -> AppError {
    match error {
        AgentDbError::CompletionBlocked(blockers) => AppError::InvalidState {
            current: format!("Agent 尚有未完成事项: {blockers:?}"),
            operation: "完成 Agent".into(),
        },
        AgentDbError::BudgetExhausted(message) => AppError::InvalidState {
            current: format!("Agent 共享预算已耗尽: {message}"),
            operation: "预留执行额度".into(),
        },
        AgentDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AgentDbError::NotFound(message) => AppError::NotFound(message),
        AgentDbError::Conflict(message) => AppError::InvalidState {
            current: message,
            operation: "执行 Agent 操作".into(),
        },
        AgentDbError::Database(error) => AppError::Database(error),
    }
}

pub(super) fn collaboration_db_error_to_app(error: CollaborationDbError) -> AppError {
    match error {
        CollaborationDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        CollaborationDbError::NotFound(message) => AppError::NotFound(message),
        CollaborationDbError::Forbidden(message) => AppError::Forbidden(message),
        CollaborationDbError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "校验 Agent 资源访问权限".into(),
        },
        CollaborationDbError::Internal(message) => AppError::Internal(anyhow::anyhow!(message)),
        CollaborationDbError::Database(error) => AppError::Database(error),
    }
}
