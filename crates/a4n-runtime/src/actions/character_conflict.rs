use crate::{
    AppState,
    actions::{
        access,
        llm_request::{
            ContextAwareJsonLlmRequestInput, PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT,
            context_aware_system_user_llm_request,
        },
        request_preview::{
            ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
            model_request_preview,
        },
    },
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::{
    chapter::chapter,
    character::{attribute_event, character_state_index, protagonist_profile},
};
use a4n_engine::llm::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_engine::prompt::request::build_json_task_user_message;
use a4n_services::chapter::analysis::summary_text;
use a4n_services::generation::{
    context_aware_chat_request_trace_json, generation_checkpoint_fingerprint_sha256,
};
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

pub type CharacterConflictProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone, Copy)]
pub struct CharacterConflictTaskScope {
    pub project_id: i64,
    pub total: u32,
}

struct CharacterConflictRequestContext {
    request: ChatRequest,
    source_ids: BTreeSet<String>,
    call_key: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharacterConflictChapterScope {
    pub start_chapter: Option<i32>,
    pub end_chapter: Option<i32>,
}

impl CharacterConflictChapterScope {
    pub fn new(start_chapter: Option<i32>, end_chapter: Option<i32>) -> Result<Self, AppError> {
        if start_chapter.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "角色一致性检查起始章节必须大于 0".to_string(),
            ));
        }
        if end_chapter.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "角色一致性检查结束章节必须大于 0".to_string(),
            ));
        }
        if let (Some(start), Some(end)) = (start_chapter, end_chapter)
            && start > end
        {
            return Err(AppError::InvalidArgument(
                "角色一致性检查起始章节不能大于结束章节".to_string(),
            ));
        }
        Ok(Self {
            start_chapter,
            end_chapter,
        })
    }

    fn to_context_value(self) -> Value {
        json!({
            "start_chapter": self.start_chapter,
            "end_chapter": self.end_chapter,
        })
    }
}

const MAX_CONFLICT_FALLBACK_CONTENT_CHARS: usize = 4_000;
const MAX_CONFLICT_TOTAL_CONTEXT_CHARS: usize = 24_000;

pub async fn character_conflict_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterConflictTaskScope, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    Ok(CharacterConflictTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn check_character_conflict(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    progress: CharacterConflictProgressReporter<'_>,
) -> Result<Vec<Value>, AppError> {
    progress(0, 4, "正在读取角色档案和章节内容")?;
    let context = build_character_conflict_request(
        state,
        user_id,
        project_id,
        name,
        start_chapter,
        end_chapter,
    )
    .await?;

    progress(1, 4, "正在构造一致性检查提示词")?;
    progress(2, 4, "正在调用模型检查角色一致性")?;
    let workflow_fingerprint = character_conflict_workflow_fingerprint(
        project_id,
        name,
        start_chapter,
        end_chapter,
        &context.request,
    )?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: character_conflict_business_key(&workflow_fingerprint)?,
            workflow_kind: "character_conflict_check".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some("check".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(context.call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let result = run_character_conflict_llm_call(
        state,
        workflow_run.id,
        workflow_fingerprint,
        context,
        progress,
    )
    .await;
    finalize_character_conflict_workflow(state, workflow_run.id, &result).await?;
    result
}

async fn run_character_conflict_llm_call(
    state: &AppState,
    workflow_run_id: i64,
    workflow_fingerprint: Value,
    mut context: CharacterConflictRequestContext,
    progress: CharacterConflictProgressReporter<'_>,
) -> Result<Vec<Value>, AppError> {
    let call_key = context.call_key.clone();
    let fingerprint_json = character_conflict_call_fingerprint_json(
        &call_key,
        &workflow_fingerprint,
        &context.request,
    )?;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
    if let Some(trace_context) = context.request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .workflow_run
        .mark_current_step(workflow_run_id, &call_key)
        .await?;
    if let Some(checkpoint) = load_completed_llm_call_checkpoint(
        state.db.clone(),
        workflow_run_id,
        &call_key,
        &fingerprint_json,
    )
    .await?
    {
        progress(3, 4, "角色冲突检查已从断点恢复，正在解析矛盾项")?;
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
        return restored_character_conflict_checkpoint(
            checkpoint_json,
            &call_key,
            &context.source_ids,
        );
    }

    let response = state
        .llm
        .chat(context.request)
        .await
        .map_err(|err| AppError::LlmError(err.to_string()))?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(
            "角色冲突检查输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次检查上下文后重试".to_string(),
        ));
    }

    progress(3, 4, "模型已返回，正在解析矛盾项")?;
    let conflicts =
        parse_conflict_output_with_source_ids(&response.message.content, &context.source_ids)?;
    let response_usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    save_completed_llm_call_checkpoint(
        state.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": conflicts.clone(),
                "parsed_output": conflicts.clone(),
                "response_content": response.message.content,
                "finish_reason": response.finish_reason,
                "usage": response_usage_json,
            }),
            artifacts_json: None,
        },
    )
    .await?;
    Ok(conflicts)
}

async fn build_character_conflict_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<CharacterConflictRequestContext, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let chapter_scope = CharacterConflictChapterScope::new(start_chapter, end_chapter)?;
    build_character_conflict_request_unchecked(state, project_id, name, chapter_scope).await
}

fn character_conflict_workflow_fingerprint(
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "character_conflict_check",
        "schema_version": "character_conflict_workflow_v1",
        "project_id": project_id,
        "character_name": name,
        "chapter_scope": {
            "start_chapter": start_chapter,
            "end_chapter": end_chapter,
        },
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn character_conflict_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "character_conflict_check:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

fn character_conflict_call_fingerprint_json(
    call_key: &str,
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "character_conflict_call_v1",
        "workflow": "character_conflict_check",
        "call_key": call_key,
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

async fn finalize_character_conflict_workflow<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<T, AppError>,
) -> Result<(), AppError> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(err) => (WORKFLOW_STATUS_FAILED, true, Some(err.to_string())),
    };
    state
        .workflow_run
        .mark_status(
            workflow_run_id,
            status,
            can_resume,
            error_message.as_deref(),
        )
        .await?;
    Ok(())
}

pub async fn preview_character_conflict_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<ModelRequestPreview, AppError> {
    let context = build_character_conflict_request(
        state,
        user_id,
        project_id,
        name,
        start_chapter,
        end_chapter,
    )
    .await?;
    Ok(model_request_preview(
        "character_conflict_check",
        "角色一致性检查 LLM 请求预览",
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "character_conflict".to_string(),
                    title: "角色一致性检查 LLM 请求".to_string(),
                    prompt_name: Some("character_conflict_check".to_string()),
                    state: "ready".to_string(),
                    note: None,
                    request: context.request,
                },
            )
            .await
            .map_err(AppError::InvalidArgument)?,
        ],
    ))
}

async fn build_character_conflict_request_unchecked(
    state: &AppState,
    project_id: i64,
    name: &str,
    chapter_scope: CharacterConflictChapterScope,
) -> Result<CharacterConflictRequestContext, AppError> {
    let profile = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name))
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("角色不存在: project_id={project_id}, name={name}"))
        })?;

    let mut chapters_query = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber);
    if let Some(start_chapter) = chapter_scope.start_chapter {
        chapters_query = chapters_query.filter(chapter::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = chapter_scope.end_chapter {
        chapters_query = chapters_query.filter(chapter::Column::ChapterNumber.lte(end_chapter));
    }
    let chapters = chapters_query.all(state.db.as_ref()).await?;
    let mut state_timeline_query = character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.eq(profile.id))
        .order_by_asc(character_state_index::Column::ChapterNumber);
    if let Some(start_chapter) = chapter_scope.start_chapter {
        state_timeline_query = state_timeline_query
            .filter(character_state_index::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = chapter_scope.end_chapter {
        state_timeline_query = state_timeline_query
            .filter(character_state_index::Column::ChapterNumber.lte(end_chapter));
    }
    let state_timeline = state_timeline_query.all(state.db.as_ref()).await?;
    let mut attribute_events_query = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile.id))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Timestamp);
    if let Some(start_chapter) = chapter_scope.start_chapter {
        attribute_events_query = attribute_events_query
            .filter(attribute_event::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = chapter_scope.end_chapter {
        attribute_events_query =
            attribute_events_query.filter(attribute_event::Column::ChapterNumber.lte(end_chapter));
    }
    let attribute_events = attribute_events_query.all(state.db.as_ref()).await?;

    let role_attr = json!({
        "source_id": "profile",
        "base_attributes": profile.base_attributes,
        "current_state": profile.current_state,
    });
    let mut chapter_context = Vec::new();
    for chapter in &chapters {
        if let Some(context) = character_conflict_chapter_context(chapter)? {
            chapter_context.push(context);
        }
    }
    let state_timeline = state_timeline
        .iter()
        .map(character_state_context)
        .collect::<Vec<_>>();
    let attribute_events = attribute_events
        .iter()
        .map(attribute_event_context)
        .collect::<Vec<_>>();
    ensure_character_conflict_has_comparison_context(
        &chapter_context,
        &state_timeline,
        &attribute_events,
    )?;
    ensure_character_conflict_context_size(
        &role_attr,
        &chapter_context,
        &state_timeline,
        &attribute_events,
    )?;
    let source_ids = character_conflict_allowed_source_ids(
        &role_attr,
        &chapter_context,
        &state_timeline,
        &attribute_events,
    );
    let chapter_context_count = chapter_context.len();
    let state_timeline_count = state_timeline.len();
    let attribute_event_count = attribute_events.len();

    let stable_context = json!({
        "chapter_scope": chapter_scope.to_context_value(),
        "character_profile": role_attr,
        "chapter_context": chapter_context,
        "character_state_timeline": state_timeline,
        "attribute_events": attribute_events
    });
    let context_value = json!({
        "task": {
            "name": "character_conflict_check",
            "instruction": "检查角色档案、角色状态时间线、属性事件与章节上下文是否存在事实、性格、关系或状态矛盾"
        },
        "input": {
            "context_refs": {
                "chapter_scope": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.chapter_scope"),
                "character_profile": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.character_profile"),
                "chapter_context": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.chapter_context"),
                "character_state_timeline": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.character_state_timeline"),
                "attribute_events": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.attribute_events")
            }
        },
        "output_contract": {
            "format": "json_array",
            "empty_result": [],
            "item_required_fields": [
                "chapter_number",
                "field",
                "expected",
                "actual",
                "expected_source_id",
                "actual_source_id",
                "severity"
            ],
            "source_id_rule": "expected_source_id 和 actual_source_id 必须从 input 中已有 source_id 选择，分别指向被期望事实和实际冲突事实；不要自造 source_id",
            "severity_values": ["minor", "moderate", "major"]
        }
    });
    let system_prompt = state
        .engine
        .render(
            "character_conflict_check",
            &HashMap::new(),
            Some(project_id),
        )
        .map_err(|err| AppError::Internal(err.into()))?;
    let user_prompt = build_json_task_user_message(
        "执行角色冲突检查",
        "只返回 output_contract 要求的严格 JSON 数组，不要输出 Markdown、代码块、解释文字或额外字段",
        &context_value,
    )?;
    let call_key = format!("character_conflict:{}:{}", profile.id, name);
    Ok(CharacterConflictRequestContext {
        request: context_aware_system_user_llm_request(
            system_prompt,
            ContextAwareJsonLlmRequestInput {
                project_id,
                workflow_run_id: None,
                workflow: "character_conflict_check".to_string(),
                step_key: "conflict_detection".to_string(),
                scope_kind: "project".to_string(),
                pack_kind: PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT.to_string(),
                source_refs_json: Some(json!({
                    "profile_id": profile.id,
                    "profile_name": &profile.name,
                    "chapter_scope": chapter_scope.to_context_value(),
                    "chapter_context_count": chapter_context_count,
                    "state_timeline_count": state_timeline_count,
                    "attribute_event_count": attribute_event_count,
                })),
                config_json: json!({
                    "ledger_version": "character_conflict_context_v1",
                    "prompt_name": "character_conflict_check",
                }),
                stable_context_json: stable_context,
                dynamic_user_prompt: user_prompt,
                temperature: Some(0.3),
                max_tokens_profile: llm_token_profiles::JSON_STANDARD.to_string(),
                call_key: Some(call_key.clone()),
            },
        )
        .map_err(AppError::InvalidArgument)?,
        source_ids,
        call_key,
    })
}

fn parse_conflict_output_with_source_ids(
    content: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<Vec<Value>, AppError> {
    let conflicts = parse_conflict_output(content)?;
    for (index, conflict) in conflicts.iter().enumerate() {
        validate_conflict_source_id(index, conflict, "expected_source_id", allowed_source_ids)?;
        validate_conflict_source_id(index, conflict, "actual_source_id", allowed_source_ids)?;
    }
    Ok(conflicts)
}

fn restored_character_conflict_checkpoint(
    checkpoint_json: Value,
    call_key: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<Vec<Value>, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:parsed_output".to_string(),
            operation: format!("恢复角色冲突检查 LLM 调用断点 {call_key}"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:response_content".to_string(),
            operation: format!("恢复角色冲突检查 LLM 调用断点 {call_key}"),
        })?;
    let reparsed = parse_conflict_output_with_source_ids(response_content, allowed_source_ids)?;
    if reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("恢复角色冲突检查 LLM 调用断点 {call_key}"),
        });
    }
    Ok(reparsed)
}

fn parse_conflict_output(content: &str) -> Result<Vec<Value>, AppError> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        AppError::InvalidArgument(format!("角色冲突检查结果必须是 JSON 数组: {err}"))
    })?;
    let conflicts = value
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("角色冲突检查结果必须是 JSON 数组".to_string()))?;

    for (index, conflict) in conflicts.iter().enumerate() {
        validate_conflict_item(index, conflict)?;
    }

    Ok(conflicts.clone())
}

fn validate_conflict_item(index: usize, conflict: &Value) -> Result<(), AppError> {
    let object = conflict
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("conflicts[{index}] 必须是 JSON 对象")))?;
    reject_unknown_conflict_fields(index, object)?;
    required_i64(conflict, "chapter_number")?;
    required_string(conflict, "field")?;
    required_present(conflict, "expected")?;
    required_present(conflict, "actual")?;
    required_string(conflict, "expected_source_id")?;
    required_string(conflict, "actual_source_id")?;
    let severity = required_string(conflict, "severity")?;
    if !matches!(severity.as_str(), "minor" | "moderate" | "major") {
        return Err(AppError::InvalidArgument(format!(
            "conflicts[{index}].severity 必须是 minor/moderate/major"
        )));
    }
    Ok(())
}

fn reject_unknown_conflict_fields(
    index: usize,
    object: &serde_json::Map<String, Value>,
) -> Result<(), AppError> {
    const ALLOWED: &[&str] = &[
        "chapter_number",
        "field",
        "expected",
        "actual",
        "expected_source_id",
        "actual_source_id",
        "severity",
    ];
    for key in object.keys() {
        if !ALLOWED.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "conflicts[{index}] 包含未支持字段 `{key}`"
            )));
        }
    }
    Ok(())
}

fn validate_conflict_source_id(
    index: usize,
    conflict: &Value,
    field: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<(), AppError> {
    let source_id = required_string(conflict, field)?;
    if allowed_source_ids.contains(&source_id) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "conflicts[{index}].{field} 必须来自本次输入 source_id: {source_id}"
        )))
    }
}

fn required_i64(value: &Value, field: &str) -> Result<i64, AppError> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 缺失或不是整数")))
}

fn required_string(value: &Value, field: &str) -> Result<String, AppError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 缺失或不是非空字符串")))
}

fn required_present<'a>(value: &'a Value, field: &str) -> Result<&'a Value, AppError> {
    value
        .get(field)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 缺失")))
}

fn character_conflict_chapter_context(ch: &chapter::Model) -> Result<Option<Value>, AppError> {
    let mut object = serde_json::Map::new();
    object.insert(
        "source_id".to_string(),
        json!(format!("chapter:{}", ch.chapter_number)),
    );
    object.insert("chapter_number".to_string(), json!(ch.chapter_number));
    if let Some(title) = ch
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        object.insert("title".to_string(), json!(title));
    }
    if let Some(summary) = ch
        .analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| !summary.is_empty())
                .map(ToOwned::to_owned)
        })
    {
        object.insert("summary".to_string(), json!(summary));
    } else if !ch.content.trim().is_empty() {
        let content = ch.content.trim();
        let content_chars = content.chars().count();
        if content_chars > MAX_CONFLICT_FALLBACK_CONTENT_CHARS {
            return Err(AppError::InvalidArgument(format!(
                "第 {} 章缺少结构化分析摘要和章节摘要，且正文长度 {} 字超过角色一致性检查兜底上限 {} 字；请先生成章节分析或补充章节摘要后重试",
                ch.chapter_number, content_chars, MAX_CONFLICT_FALLBACK_CONTENT_CHARS
            )));
        }
        object.insert("content".to_string(), json!(content));
    } else {
        return Ok(None);
    }
    if let Some(analysis) = ch.analysis_data.as_ref() {
        let mut analysis_context = serde_json::Map::new();
        if let Some(key_events) = analysis.get("key_events") {
            analysis_context.insert("key_events".to_string(), key_events.clone());
        }
        if let Some(metadata) = analysis.get("metadata") {
            analysis_context.insert("metadata".to_string(), metadata.clone());
        }
        if !analysis_context.is_empty() {
            object.insert("analysis".to_string(), Value::Object(analysis_context));
        }
    }
    Ok(Some(Value::Object(object)))
}

fn character_state_context(state: &character_state_index::Model) -> Value {
    json!({
        "source_id": format!("state:{}", state.chapter_number),
        "chapter_number": state.chapter_number,
        "state_summary": state.state_summary,
        "key_attributes": state.key_attributes,
        "emotional_state": state.emotional_state.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "current_location": state.current_location.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "current_goal": state.current_goal.as_deref().map(str::trim).filter(|value| !value.is_empty()),
    })
}

fn attribute_event_context(event: &attribute_event::Model) -> Value {
    json!({
        "source_id": format!("attribute_event:{}", event.id),
        "chapter_number": event.chapter_number,
        "category": event.category,
        "key": event.key,
        "operation": event.operation,
        "change_description": event.change_description,
        "old_value": event.old_value,
        "new_value": event.new_value,
        "cause": event.cause,
        "evidence": event.evidence,
    })
}

fn character_conflict_allowed_source_ids(
    role_attr: &Value,
    chapter_context: &[Value],
    state_timeline: &[Value],
    attribute_events: &[Value],
) -> BTreeSet<String> {
    let mut source_ids = BTreeSet::new();
    collect_top_level_source_id(role_attr, &mut source_ids);
    for source in chapter_context
        .iter()
        .chain(state_timeline)
        .chain(attribute_events)
    {
        collect_top_level_source_id(source, &mut source_ids);
    }
    source_ids
}

fn ensure_character_conflict_has_comparison_context(
    chapter_context: &[Value],
    state_timeline: &[Value],
    attribute_events: &[Value],
) -> Result<(), AppError> {
    if chapter_context.is_empty() && state_timeline.is_empty() && attribute_events.is_empty() {
        Err(AppError::InvalidArgument(
            "角色一致性检查缺少可对照上下文：没有可用章节上下文、角色状态时间线或属性事件；请先生成章节分析、同步角色档案或补充可检查上下文后重试"
                .to_string(),
        ))
    } else {
        Ok(())
    }
}

fn ensure_character_conflict_context_size(
    role_attr: &Value,
    chapter_context: &[Value],
    state_timeline: &[Value],
    attribute_events: &[Value],
) -> Result<(), AppError> {
    let total_chars = character_conflict_context_chars(
        role_attr,
        chapter_context,
        state_timeline,
        attribute_events,
    );
    if total_chars > MAX_CONFLICT_TOTAL_CONTEXT_CHARS {
        Err(AppError::InvalidArgument(format!(
            "角色一致性检查全量上下文过大：当前 {} 字，超过上限 {} 字；请先生成章节分析或章节摘要以减少正文兜底，或改用更小范围的检查入口后重试",
            total_chars, MAX_CONFLICT_TOTAL_CONTEXT_CHARS
        )))
    } else {
        Ok(())
    }
}

fn character_conflict_context_chars(
    role_attr: &Value,
    chapter_context: &[Value],
    state_timeline: &[Value],
    attribute_events: &[Value],
) -> usize {
    json!({
        "character_profile": role_attr,
        "chapter_context": chapter_context,
        "character_state_timeline": state_timeline,
        "attribute_events": attribute_events
    })
    .to_string()
    .chars()
    .count()
}

fn collect_top_level_source_id(value: &Value, out: &mut BTreeSet<String>) {
    if let Some(source_id) = value
        .get("source_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        out.insert(source_id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_chapter(summary: Option<&str>, content: &str) -> chapter::Model {
        let now = NaiveDate::from_ymd_opt(2026, 5, 31)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        chapter::Model {
            id: 1,
            project_id: 9,
            outline_id: None,
            chapter_number: 3,
            title: Some("旧城".to_string()),
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: "draft".to_string(),
            summary: summary.map(ToOwned::to_owned),
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn character_conflict_chapter_context_uses_summary_before_content() {
        let chapter = sample_chapter(Some("主角进入旧城。"), "完整正文不应重复塞入");

        let context = character_conflict_chapter_context(&chapter)
            .expect("context should build")
            .expect("chapter should be included");

        assert_eq!(context["source_id"], json!("chapter:3"));
        assert_eq!(context["summary"], json!("主角进入旧城。"));
        assert!(context.get("content").is_none());
        assert!(context.get("content_preview").is_none());
    }

    #[test]
    fn character_conflict_chapter_context_uses_short_content_when_summary_missing() {
        let content = "陈恪走进旧城，看见魏征站在雨里。";
        let chapter = sample_chapter(None, &content);

        let context = character_conflict_chapter_context(&chapter)
            .expect("short content should build")
            .expect("chapter should be included");

        assert_eq!(context["source_id"], json!("chapter:3"));
        assert_eq!(context["content"], json!(content));
        assert!(context.get("content_preview").is_none());
    }

    #[test]
    fn character_conflict_chapter_context_rejects_long_content_without_summary() {
        let content = "陈恪走进旧城，看见魏征站在雨里。"
            .repeat(MAX_CONFLICT_FALLBACK_CONTENT_CHARS / 10 + 10);
        let chapter = sample_chapter(None, &content);

        let err = character_conflict_chapter_context(&chapter)
            .expect_err("long content without summary should fail");

        let message = err.to_string();
        assert!(message.contains("缺少结构化分析摘要和章节摘要"));
        assert!(message.contains("超过角色一致性检查兜底上限"));
    }

    #[test]
    fn character_conflict_allowed_source_ids_uses_only_top_level_inputs() {
        let role_attr = json!({"source_id": "profile"});
        let chapter_context = vec![json!({
            "source_id": "chapter:3",
            "analysis": {
                "key_events": [
                    {"source_id": "legacy_event:1", "summary": "旧嵌套来源不应被允许"}
                ]
            }
        })];
        let state_timeline = vec![json!({"source_id": "state:3"})];
        let attribute_events = vec![json!({"source_id": "attribute_event:9"})];

        let source_ids = character_conflict_allowed_source_ids(
            &role_attr,
            &chapter_context,
            &state_timeline,
            &attribute_events,
        );

        assert!(source_ids.contains("profile"));
        assert!(source_ids.contains("chapter:3"));
        assert!(source_ids.contains("state:3"));
        assert!(source_ids.contains("attribute_event:9"));
        assert!(!source_ids.contains("legacy_event:1"));
    }

    #[test]
    fn restored_character_conflict_checkpoint_revalidates_response_content() {
        let mut allowed_source_ids = BTreeSet::new();
        allowed_source_ids.insert("profile".to_string());
        allowed_source_ids.insert("chapter:3".to_string());
        let raw = r#"[{
            "chapter_number": 3,
            "field": "goal",
            "expected": "留在旧城",
            "actual": "已经离开旧城",
            "expected_source_id": "profile",
            "actual_source_id": "chapter:3",
            "severity": "major"
        }]"#;
        let output = parse_conflict_output_with_source_ids(raw, &allowed_source_ids)
            .expect("raw checkpoint response should parse");

        let restored = restored_character_conflict_checkpoint(
            json!({
                "output": output.clone(),
                "parsed_output": output,
                "response_content": raw,
            }),
            "character_conflict:1:陈恪",
            &allowed_source_ids,
        )
        .expect("valid checkpoint should restore");

        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0]["severity"], json!("major"));
    }

    #[test]
    fn restored_character_conflict_checkpoint_rejects_mismatched_output() {
        let mut allowed_source_ids = BTreeSet::new();
        allowed_source_ids.insert("profile".to_string());
        allowed_source_ids.insert("chapter:3".to_string());
        let raw = r#"[{
            "chapter_number": 3,
            "field": "goal",
            "expected": "留在旧城",
            "actual": "已经离开旧城",
            "expected_source_id": "profile",
            "actual_source_id": "chapter:3",
            "severity": "major"
        }]"#;

        let err = restored_character_conflict_checkpoint(
            json!({
                "output": [],
                "parsed_output": [],
                "response_content": raw,
            }),
            "character_conflict:1:陈恪",
            &allowed_source_ids,
        )
        .expect_err("checkpoint output mismatch must fail");

        assert!(
            err.to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }

    #[test]
    fn character_conflict_requires_comparison_context() {
        let err = ensure_character_conflict_has_comparison_context(&[], &[], &[])
            .expect_err("profile-only conflict check should fail");

        let message = err.to_string();
        assert!(message.contains("缺少可对照上下文"));
        assert!(message.contains("角色状态时间线"));
    }

    #[test]
    fn character_conflict_allows_any_comparison_context() {
        let chapter_context = vec![json!({"source_id": "chapter:3"})];
        ensure_character_conflict_has_comparison_context(&chapter_context, &[], &[])
            .expect("chapter context is sufficient");

        let state_timeline = vec![json!({"source_id": "state:3"})];
        ensure_character_conflict_has_comparison_context(&[], &state_timeline, &[])
            .expect("state timeline is sufficient");

        let attribute_events = vec![json!({"source_id": "attribute_event:9"})];
        ensure_character_conflict_has_comparison_context(&[], &[], &attribute_events)
            .expect("attribute events are sufficient");
    }

    #[test]
    fn character_conflict_allows_context_within_size_limit() {
        let role_attr = json!({"source_id": "profile", "base_attributes": {"role": "主角"}});
        let chapter_context = vec![json!({
            "source_id": "chapter:3",
            "chapter_number": 3,
            "summary": "主角进入旧城。"
        })];

        ensure_character_conflict_context_size(&role_attr, &chapter_context, &[], &[])
            .expect("small conflict context should pass");
    }

    #[test]
    fn character_conflict_rejects_oversized_total_context() {
        let role_attr = json!({"source_id": "profile"});
        let chapter_context = vec![json!({
            "source_id": "chapter:3",
            "chapter_number": 3,
            "summary": "角色状态摘要过长。".repeat(MAX_CONFLICT_TOTAL_CONTEXT_CHARS)
        })];

        let err = ensure_character_conflict_context_size(&role_attr, &chapter_context, &[], &[])
            .expect_err("oversized conflict context should fail");

        let message = err.to_string();
        assert!(message.contains("全量上下文过大"));
        assert!(message.contains("超过上限"));
    }

    #[test]
    fn character_conflict_chapter_scope_accepts_full_and_range() {
        assert_eq!(
            CharacterConflictChapterScope::new(None, None).expect("full scope should pass"),
            CharacterConflictChapterScope {
                start_chapter: None,
                end_chapter: None,
            }
        );
        assert_eq!(
            CharacterConflictChapterScope::new(Some(2), Some(5))
                .expect("bounded scope should pass"),
            CharacterConflictChapterScope {
                start_chapter: Some(2),
                end_chapter: Some(5),
            }
        );
    }

    #[test]
    fn character_conflict_chapter_scope_rejects_invalid_range() {
        let err = CharacterConflictChapterScope::new(Some(0), None)
            .expect_err("zero start chapter should fail");
        assert!(err.to_string().contains("起始章节必须大于 0"));

        let err = CharacterConflictChapterScope::new(None, Some(-1))
            .expect_err("negative end chapter should fail");
        assert!(err.to_string().contains("结束章节必须大于 0"));

        let err = CharacterConflictChapterScope::new(Some(5), Some(2))
            .expect_err("reversed range should fail");
        assert!(err.to_string().contains("起始章节不能大于结束章节"));
    }

    #[test]
    fn parse_conflict_output_accepts_empty_array() {
        let conflicts = parse_conflict_output("[]").expect("empty conflicts should parse");
        assert!(conflicts.is_empty());
    }

    #[test]
    fn parse_conflict_output_rejects_embedded_json() {
        let err = parse_conflict_output(r#"结果如下：[{"chapter_number":1}]"#)
            .expect_err("embedded JSON must fail");
        assert!(err.to_string().contains("JSON 数组"));
    }

    #[test]
    fn parse_conflict_output_rejects_invalid_item() {
        let err = parse_conflict_output(
            r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"critical"}]"#,
        )
        .expect_err("invalid severity must fail");
        assert!(err.to_string().contains("severity"));
    }

    #[test]
    fn parse_conflict_output_rejects_missing_source_ids() {
        let err = parse_conflict_output(
            r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","severity":"major"}]"#,
        )
        .expect_err("source ids are required");

        assert!(err.to_string().contains("expected_source_id"));
    }

    #[test]
    fn parse_conflict_output_rejects_unknown_item_fields() {
        let err = parse_conflict_output(
            r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"major","notes":"多余说明"}]"#,
        )
        .expect_err("unknown conflict fields should fail");

        assert!(err.to_string().contains("未支持字段 `notes`"));
    }

    #[test]
    fn parse_conflict_output_with_source_ids_accepts_selected_sources() {
        let allowed = ["profile".to_string(), "state:1".to_string()]
            .into_iter()
            .collect();
        let conflicts = parse_conflict_output_with_source_ids(
            r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"major"}]"#,
            &allowed,
        )
        .expect("selected source ids should pass");

        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn parse_conflict_output_with_source_ids_rejects_unknown_source() {
        let allowed = ["profile".to_string()].into_iter().collect();
        let err = parse_conflict_output_with_source_ids(
            r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:9","severity":"major"}]"#,
            &allowed,
        )
        .expect_err("unknown source id must fail");

        let message = err.to_string();
        assert!(message.contains("actual_source_id"));
        assert!(message.contains("本次输入 source_id"));
    }

    #[test]
    fn parse_conflict_output_rejects_nested_analysis_source_ids_not_in_top_level_inputs() {
        let role_attr = json!({"source_id": "profile"});
        let chapter_context = vec![json!({
            "source_id": "chapter:3",
            "analysis": {"key_events": [{"source_id": "legacy_event:1"}]}
        })];
        let source_ids =
            character_conflict_allowed_source_ids(&role_attr, &chapter_context, &[], &[]);

        let err = parse_conflict_output_with_source_ids(
            r#"[{"chapter_number":3,"field":"appearance","expected":"黑衣","actual":"白衣","expected_source_id":"legacy_event:1","actual_source_id":"chapter:3","severity":"minor"}]"#,
            &source_ids,
        )
        .expect_err("nested analysis source ids must not be allowed");

        let message = err.to_string();
        assert!(message.contains("expected_source_id"));
        assert!(message.contains("本次输入 source_id"));
    }
}
