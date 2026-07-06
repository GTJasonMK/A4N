use crate::AppState;
use crate::actions::{
    access,
    llm_request::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT,
        context_aware_system_user_llm_request,
    },
    request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter, chapter_outline, chapter_scene};
use a4n_engine::llm::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::request::build_json_task_user_message;
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
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

pub type ChapterSceneProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSceneSummary {
    pub id: i64,
    pub chapter_id: i64,
    pub scene_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub summary: Option<String>,
    pub scene_type: String,
    pub characters_present: Option<Value>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
    pub transition_from_previous: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChapterSceneInput {
    pub scene_id: i64,
    pub content: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub scene_type: Option<String>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReorderScenesInput {
    pub order: Vec<(i64, i32)>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterSceneInput {
    pub chapter_id: i64,
    pub scene_number: i32,
    pub scene_prompt: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterSceneTaskScope {
    pub project_id: i64,
    pub total: u32,
}

struct ChapterSceneGenerationRequestContext {
    project_id: i64,
    chapter_number: i32,
    request: ChatRequest,
    call_key: String,
}

fn chapter_scene_summary(scene: &chapter_scene::Model) -> ChapterSceneSummary {
    ChapterSceneSummary {
        id: scene.id,
        chapter_id: scene.chapter_id,
        scene_number: scene.scene_number,
        title: scene.title.clone(),
        content: scene.content.clone(),
        word_count: scene.word_count,
        summary: scene.summary.clone(),
        scene_type: scene.scene_type.clone(),
        characters_present: scene.characters_present.clone(),
        location: scene.location.clone(),
        timeline_position: scene.timeline_position.clone(),
        transition_from_previous: scene.transition_from_previous.clone(),
        created_at: Some(scene.created_at.to_string()),
        updated_at: Some(scene.updated_at.to_string()),
    }
}

pub async fn list_chapter_scenes(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    verify_chapter_access(state, user_id, chapter_id).await?;

    let scenes = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_scene::Column::SceneNumber)
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    Ok(scenes.iter().map(chapter_scene_summary).collect())
}

pub async fn update_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterSceneInput,
) -> Result<ChapterSceneSummary, AppError> {
    let scene = verify_scene_access(state, user_id, input.scene_id).await?;
    let now = chrono::Utc::now().naive_utc();
    let mut active: chapter_scene::ActiveModel = scene.into();

    if let Some(content) = input.content {
        let content = required_updated_text(&content, "场景正文")?;
        active.content = Set(content.clone());
        active.word_count = Set(content.chars().count() as i32);
    }
    if let Some(title) = input.title {
        active.title = Set(trimmed_optional_text(&title));
    }
    if let Some(summary) = input.summary {
        active.summary = Set(trimmed_optional_text(&summary));
    }
    if let Some(scene_type) = input.scene_type {
        active.scene_type = Set(required_updated_text(&scene_type, "场景类型")?);
    }
    if let Some(location) = input.location {
        active.location = Set(trimmed_optional_text(&location));
    }
    if let Some(timeline_position) = input.timeline_position {
        active.timeline_position = Set(trimmed_optional_text(&timeline_position));
    }
    active.updated_at = Set(now);

    let updated = active
        .update(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    Ok(chapter_scene_summary(&updated))
}

pub async fn delete_chapter_scene(
    state: &AppState,
    user_id: i64,
    scene_id: i64,
) -> Result<(), AppError> {
    verify_scene_access(state, user_id, scene_id).await?;
    let result = chapter_scene::Entity::delete_by_id(scene_id)
        .exec(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    if result.rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "场景不存在: scene_id={scene_id}"
        )));
    }
    Ok(())
}

pub async fn reorder_chapter_scenes(
    state: &AppState,
    user_id: i64,
    input: ReorderScenesInput,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    if input.order.is_empty() {
        return Ok(Vec::new());
    }

    let txn = state.db.begin().await.map_err(AppError::Database)?;
    let mut scenes = Vec::with_capacity(input.order.len());
    let mut seen_scene_ids = HashSet::new();
    let mut seen_numbers = HashSet::new();
    let mut chapter_id = None;

    for (scene_id, new_number) in &input.order {
        if *new_number <= 0 {
            return Err(AppError::InvalidArgument("场景序号必须为正数".to_string()));
        }
        if !seen_scene_ids.insert(*scene_id) {
            return Err(AppError::InvalidArgument(format!(
                "场景排序包含重复场景: scene_id={scene_id}"
            )));
        }
        if !seen_numbers.insert(*new_number) {
            return Err(AppError::InvalidArgument(format!(
                "场景排序包含重复序号: scene_number={new_number}"
            )));
        }
        let scene = chapter_scene::Entity::find_by_id(*scene_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("场景不存在: scene_id={scene_id}")))?;
        match chapter_id {
            Some(id) if id != scene.chapter_id => {
                return Err(AppError::InvalidArgument(
                    "不能跨章节调整场景顺序".to_string(),
                ));
            }
            None => chapter_id = Some(scene.chapter_id),
            _ => {}
        }
        scenes.push((scene, *new_number));
    }

    let chapter_id =
        chapter_id.ok_or_else(|| AppError::InvalidArgument("场景排序为空".to_string()))?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(&txn)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
    access::verify_project_editor(state, user_id, chapter.project_id).await?;

    let target_numbers = seen_numbers.iter().copied().collect::<Vec<_>>();
    let conflicts = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(chapter_id))
        .filter(chapter_scene::Column::SceneNumber.is_in(target_numbers))
        .all(&txn)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .filter(|scene| !seen_scene_ids.contains(&scene.id))
        .collect::<Vec<_>>();
    if let Some(conflict) = conflicts.first() {
        return Err(AppError::InvalidArgument(format!(
            "场景序号 {} 已被场景 #{} 占用",
            conflict.scene_number, conflict.id
        )));
    }

    let all_scenes = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(chapter_id))
        .all(&txn)
        .await
        .map_err(AppError::Database)?;
    let min_scene_number = all_scenes
        .iter()
        .map(|scene| scene.scene_number)
        .min()
        .unwrap_or_default();
    let temp_start = min_scene_number
        .min(0)
        .saturating_sub(scenes.len() as i32)
        .saturating_sub(1);
    let now = chrono::Utc::now().naive_utc();

    for (idx, (scene, _)) in scenes.iter().enumerate() {
        let mut active: chapter_scene::ActiveModel = scene.clone().into();
        active.scene_number = Set(temp_start + idx as i32);
        active.updated_at = Set(now);
        active.update(&txn).await.map_err(AppError::Database)?;
    }

    let mut updated_scenes = Vec::with_capacity(scenes.len());
    for (scene, new_number) in scenes {
        let mut active: chapter_scene::ActiveModel = scene.into();
        active.scene_number = Set(new_number);
        active.updated_at = Set(now);
        let updated = active.update(&txn).await.map_err(AppError::Database)?;
        updated_scenes.push(chapter_scene_summary(&updated));
    }

    txn.commit().await.map_err(AppError::Database)?;

    updated_scenes.sort_by_key(|scene| scene.scene_number);
    Ok(updated_scenes)
}

pub async fn generate_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterSceneInput,
    progress: ChapterSceneProgressReporter<'_>,
) -> Result<ChapterSceneSummary, AppError> {
    progress(0, 4, "正在读取章节和场景参数")?;
    let context = build_scene_generation_request_for_input(state, user_id, &input).await?;

    progress(1, 4, "正在构造场景生成提示词")?;
    progress(2, 4, "正在调用模型生成场景正文")?;
    let workflow_fingerprint = chapter_scene_workflow_fingerprint(&input, &context.request)?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: chapter_scene_business_key(&workflow_fingerprint)?,
            workflow_kind: "chapter_scene_generation".to_string(),
            project_id: Some(context.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(input.chapter_id),
            chapter_id: Some(input.chapter_id),
            mode: Some("scene".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(context.call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let result = async {
        let content = run_chapter_scene_llm_call(
            state,
            workflow_run.id,
            workflow_fingerprint,
            context,
            progress,
        )
        .await?;
        progress(3, 4, "模型已返回，正在保存场景")?;
        let scene = save_generated_chapter_scene(state, &input, &content).await?;
        progress(4, 4, "场景已保存")?;
        Ok(chapter_scene_summary(&scene))
    }
    .await;
    finalize_chapter_scene_workflow(state, workflow_run.id, &result).await?;
    result
}

async fn run_chapter_scene_llm_call(
    state: &AppState,
    workflow_run_id: i64,
    workflow_fingerprint: Value,
    mut context: ChapterSceneGenerationRequestContext,
    progress: ChapterSceneProgressReporter<'_>,
) -> Result<String, AppError> {
    let call_key = context.call_key.clone();
    let fingerprint_json =
        chapter_scene_call_fingerprint_json(&call_key, &workflow_fingerprint, &context.request)?;
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
        progress(3, 4, "场景正文已从断点恢复，正在保存场景")?;
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
        return restored_chapter_scene_checkpoint(checkpoint_json, &call_key);
    }

    let response = state
        .llm
        .chat(context.request)
        .await
        .map_err(|err| AppError::LlmError(err.to_string()))?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(
            "场景正文生成输出达到当前 max_tokens 上限，正文可能被截断；请在设置页调大“标准”输出档位，或缩短本次场景目标后重试".to_string(),
        ));
    }
    let content = normalize_scene_generation_output(&response.message.content)?;
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
                "output": content.clone(),
                "parsed_output": content.clone(),
                "response_content": response.message.content,
                "finish_reason": response.finish_reason,
                "usage": response_usage_json,
            }),
            artifacts_json: None,
        },
    )
    .await?;
    Ok(content)
}

async fn save_generated_chapter_scene(
    state: &AppState,
    input: &GenerateChapterSceneInput,
    content: &str,
) -> Result<chapter_scene::Model, AppError> {
    let chapter = chapter::Entity::find_by_id(input.chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::NotFound(format!("章节不存在: chapter_id={}", input.chapter_id))
        })?;
    let word_count = content.chars().count() as i32;
    let now = chrono::Utc::now().naive_utc();

    let existing = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(input.chapter_id))
        .filter(chapter_scene::Column::SceneNumber.eq(input.scene_number))
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    let scene = if let Some(existing_scene) = existing {
        let mut active: chapter_scene::ActiveModel = existing_scene.into();
        active.content = Set(content.to_string());
        active.word_count = Set(word_count);
        active.updated_at = Set(now);
        active
            .update(state.db.as_ref())
            .await
            .map_err(AppError::Database)?
    } else {
        chapter_scene::ActiveModel {
            chapter_id: Set(input.chapter_id),
            scene_number: Set(input.scene_number),
            title: Set(Some(format!(
                "第{}章-场景{}",
                chapter.chapter_number, input.scene_number
            ))),
            content: Set(content.to_string()),
            word_count: Set(word_count),
            summary: Set(None),
            scene_type: Set("development".to_string()),
            characters_present: Set(None),
            location: Set(None),
            timeline_position: Set(None),
            transition_from_previous: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
    };
    Ok(scene)
}

pub async fn generate_chapter_scene_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ChapterSceneTaskScope, AppError> {
    let ch = verify_chapter_access(state, user_id, input.chapter_id).await?;
    required_scene_generation_inputs(&ch, input)?;
    Ok(ChapterSceneTaskScope {
        project_id: ch.project_id,
        total: 4,
    })
}

fn chapter_scene_workflow_fingerprint(
    input: &GenerateChapterSceneInput,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "chapter_scene_generation",
        "schema_version": "chapter_scene_generation_workflow_v1",
        "chapter_id": input.chapter_id,
        "scene_number": input.scene_number,
        "scene_prompt": input.scene_prompt.trim(),
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn chapter_scene_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "chapter_scene_generation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

fn chapter_scene_call_fingerprint_json(
    call_key: &str,
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "chapter_scene_generation_call_v1",
        "workflow": "chapter_scene_generation",
        "call_key": call_key,
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

async fn finalize_chapter_scene_workflow<T>(
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

fn restored_chapter_scene_checkpoint(
    checkpoint_json: Value,
    call_key: &str,
) -> Result<String, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:parsed_output".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:response_content".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        })?;
    let reparsed = normalize_scene_generation_output(response_content)?;
    if reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        });
    }
    Ok(reparsed)
}

async fn build_scene_generation_request_for_input(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ChapterSceneGenerationRequestContext, AppError> {
    let ch = verify_chapter_access(state, user_id, input.chapter_id).await?;
    let (title, summary, scene_prompt) = required_scene_generation_inputs(&ch, input)?;
    let outline = match ch.outline_id {
        Some(outline_id) => chapter_outline::Entity::find_by_id(outline_id)
            .one(state.db.as_ref())
            .await
            .map_err(AppError::Database)?,
        None => chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(ch.project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(ch.chapter_number))
            .one(state.db.as_ref())
            .await
            .map_err(AppError::Database)?,
    };
    let existing_scenes = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(input.chapter_id))
        .order_by_asc(chapter_scene::Column::SceneNumber)
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    let generation_context =
        build_scene_generation_context(&ch, outline.as_ref(), &existing_scenes, input.scene_number);
    let request = build_scene_generation_request(
        state.engine.as_ref(),
        ch.project_id,
        ch.chapter_number,
        title,
        summary,
        Some(&generation_context),
        input.scene_number,
        scene_prompt,
    )?;
    Ok(ChapterSceneGenerationRequestContext {
        project_id: ch.project_id,
        chapter_number: ch.chapter_number,
        call_key: format!(
            "chapter_scene_generation:{}:{}",
            ch.chapter_number, input.scene_number
        ),
        request,
    })
}

pub async fn preview_chapter_scene_request(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ModelRequestPreview, AppError> {
    let context = build_scene_generation_request_for_input(state, user_id, input).await?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: "chapter_scene_generation".to_string(),
            title: format!(
                "第 {} 章 · 场景 {}",
                context.chapter_number, input.scene_number
            ),
            prompt_name: Some("chapter_scene_generation".to_string()),
            state: "ready".to_string(),
            note: None,
            request: context.request,
        },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    Ok(model_request_preview(
        "chapter_scene_generation",
        format!("第 {} 章场景生成请求预览", context.chapter_number),
        vec![step],
    ))
}

async fn verify_scene_access(
    state: &AppState,
    user_id: i64,
    scene_id: i64,
) -> Result<chapter_scene::Model, AppError> {
    let scene = chapter_scene::Entity::find_by_id(scene_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("场景不存在: scene_id={scene_id}")))?;
    verify_chapter_access(state, user_id, scene.chapter_id).await?;
    Ok(scene)
}

async fn verify_chapter_access(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, AppError> {
    let ch = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
    access::verify_project_editor(state, user_id, ch.project_id).await?;
    Ok(ch)
}

fn required_scene_prompt_text<'a>(value: &'a str, field: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!(
            "生成章节场景缺少{field}"
        )))
    } else {
        Ok(value)
    }
}

fn trimmed_optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn required_updated_text(value: &str, field: &str) -> Result<String, AppError> {
    trimmed_optional_text(value)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field}不能为空")))
}

fn required_scene_generation_inputs<'a>(
    ch: &'a chapter::Model,
    input: &'a GenerateChapterSceneInput,
) -> Result<(&'a str, &'a str, &'a str), AppError> {
    ensure_scene_number(input.scene_number)?;
    let title = required_chapter_prompt_text(
        ch.title.as_deref(),
        "标题",
        "生成章节场景",
        ch.chapter_number,
    )?;
    let summary = required_chapter_prompt_text(
        ch.summary.as_deref(),
        "摘要",
        "生成章节场景",
        ch.chapter_number,
    )?;
    let scene_prompt = required_scene_prompt_text(&input.scene_prompt, "场景引导")?;
    Ok((title, summary, scene_prompt))
}

fn required_chapter_prompt_text<'a>(
    value: Option<&'a str>,
    field: &str,
    action: &str,
    chapter_number: i32,
) -> Result<&'a str, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{action}需要第 {chapter_number} 章已有{field}"))
        })
}

fn ensure_scene_number(scene_number: i32) -> Result<(), AppError> {
    if scene_number <= 0 {
        return Err(AppError::InvalidArgument("场景编号必须大于 0".to_string()));
    }
    Ok(())
}

#[cfg(test)]
fn build_scene_generation_user_prompt(
    chapter_number: i32,
    title: &str,
    summary: &str,
    generation_context: Option<&Value>,
    scene_number: i32,
    scene_prompt: &str,
) -> Result<String, AppError> {
    ensure_scene_number(scene_number)?;
    let scene_prompt = required_scene_prompt_text(scene_prompt, "场景引导")?;
    let payload = json!({
        "chapter": {
            "chapter_number": chapter_number,
            "title": title,
            "summary": summary,
            "available_context": generation_context.cloned().unwrap_or_else(|| json!({})),
        },
        "scene": {
            "scene_number": scene_number,
            "prompt": scene_prompt,
        },
        "output_contract": {
            "format": "plain_text",
            "requirements": [
                "只输出场景正文",
                "不要包含标题、编号、Markdown 或解释文字",
                "正文必须承接 chapter.summary，并落实 scene.prompt"
            ],
        },
    });
    build_json_task_user_message(
        "撰写章节场景",
        "只返回 output_contract 要求的场景正文，不要输出其他内容",
        &payload,
    )
}

fn build_scene_generation_manifest_user_prompt(
    scene_number: i32,
    scene_prompt: &str,
) -> Result<String, AppError> {
    ensure_scene_number(scene_number)?;
    let scene_prompt = required_scene_prompt_text(scene_prompt, "场景引导")?;
    let payload = json!({
        "task": {
            "name": "chapter_scene_generation",
            "instruction": "基于稳定上下文中的章节摘要、章节正文/大纲和已有场景，撰写目标场景正文"
        },
        "input": {
            "context_refs": {
                "chapter": format!("稳定上下文包 {PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT}.chapter"),
                "available_context": format!("稳定上下文包 {PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT}.available_context")
            },
            "scene": {
                "scene_number": scene_number,
                "prompt": scene_prompt,
            },
        },
        "output_contract": {
            "format": "plain_text",
            "requirements": [
                "只输出场景正文",
                "不要包含标题、编号、Markdown 或解释文字",
                "正文必须承接稳定上下文中的 chapter.summary，并落实 scene.prompt"
            ],
        },
    });
    build_json_task_user_message(
        "撰写章节场景",
        "只返回 output_contract 要求的场景正文，不要输出其他内容",
        &payload,
    )
}

fn scene_generation_stable_context(
    chapter_number: i32,
    title: &str,
    summary: &str,
    generation_context: Option<&Value>,
) -> Value {
    json!({
        "chapter": {
            "chapter_number": chapter_number,
            "title": title,
            "summary": summary,
        },
        "available_context": generation_context.cloned().unwrap_or_else(|| json!({})),
    })
}

fn build_scene_generation_request(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    chapter_number: i32,
    title: &str,
    summary: &str,
    generation_context: Option<&Value>,
    scene_number: i32,
    scene_prompt: &str,
) -> Result<ChatRequest, AppError> {
    let variables = std::collections::HashMap::from([
        ("chapter_number".to_string(), json!(chapter_number)),
        ("chapter_title".to_string(), json!(title)),
        ("scene_number".to_string(), json!(scene_number)),
    ]);
    let system_prompt = prompt_engine
        .render("chapter_scene_generation", &variables, Some(project_id))
        .map_err(|err| AppError::Internal(err.into()))?;
    let user_prompt = build_scene_generation_manifest_user_prompt(scene_number, scene_prompt)?;
    context_aware_system_user_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id,
            workflow_run_id: None,
            workflow: "chapter_scene_generation".to_string(),
            step_key: "scene_generation".to_string(),
            scope_kind: "chapter".to_string(),
            pack_kind: PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "chapter_number": chapter_number,
                "chapter_title": title,
                "scene_number": scene_number,
            })),
            config_json: json!({
                "ledger_version": "chapter_scene_generation_context_v1",
                "prompt_name": "chapter_scene_generation",
                "chapter_number": chapter_number,
            }),
            stable_context_json: scene_generation_stable_context(
                chapter_number,
                title,
                summary,
                generation_context,
            ),
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.8),
            max_tokens_profile: llm_token_profiles::STANDARD.to_string(),
            call_key: Some(format!(
                "chapter_scene_generation:{chapter_number}:{scene_number}"
            )),
        },
    )
    .map_err(AppError::InvalidArgument)
}

fn normalize_scene_generation_output(content: &str) -> Result<String, AppError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 未返回可用场景正文".to_string(),
        ));
    }
    if looks_like_structured_or_fenced_scene_response(content) {
        return Err(AppError::InvalidArgument(format!(
            "场景正文 LLM 响应必须是纯文本正文，不能返回 JSON、Markdown 代码块或思考标签；输出预览：{}",
            scene_response_preview(content)
        )));
    }
    Ok(content.to_string())
}

fn looks_like_structured_or_fenced_scene_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}

fn scene_response_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let prefix = trimmed.chars().take(180).collect::<String>();
    if trimmed.chars().count() <= 260 {
        return prefix;
    }
    let suffix = trimmed
        .chars()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix} … {suffix}")
}

fn scene_context_value(scene: &chapter_scene::Model) -> Value {
    json!({
        "scene_number": scene.scene_number,
        "title": scene.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": scene.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "scene_type": scene.scene_type,
        "characters_present": scene.characters_present.clone(),
        "location": scene.location.as_deref().map(str::trim).filter(|location| !location.is_empty()),
        "timeline_position": scene.timeline_position.as_deref().map(str::trim).filter(|position| !position.is_empty()),
        "content": scene.content,
    })
}

fn outline_context_value(outline: &chapter_outline::Model) -> Value {
    json!({
        "chapter_number": outline.chapter_number,
        "title": outline.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": outline.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "beats": outline.beats.clone(),
        "key_scenes": outline.key_scenes.clone(),
        "pacing_score": outline.pacing_score,
        "foreshadowing_setup": outline.foreshadowing_setup.clone(),
        "foreshadowing_payoff": outline.foreshadowing_payoff.clone(),
        "characters_involved": outline.characters_involved.clone(),
        "word_count_target": outline.word_count_target,
    })
}

fn build_scene_generation_context(
    chapter: &chapter::Model,
    outline: Option<&chapter_outline::Model>,
    scenes: &[chapter_scene::Model],
    target_scene_number: i32,
) -> Value {
    let mut object = serde_json::Map::new();
    if chapter.has_content() {
        object.insert("chapter_content".to_string(), json!(chapter.content));
    }
    if let Some(outline) = outline {
        object.insert(
            "chapter_outline".to_string(),
            outline_context_value(outline),
        );
    }

    let existing_scenes = scenes
        .iter()
        .filter(|scene| scene.scene_number != target_scene_number)
        .map(scene_context_value)
        .collect::<Vec<_>>();
    if !existing_scenes.is_empty() {
        object.insert("existing_scenes".to_string(), Value::Array(existing_scenes));
    }
    if let Some(target_scene) = scenes
        .iter()
        .find(|scene| scene.scene_number == target_scene_number)
    {
        object.insert(
            "existing_target_scene".to_string(),
            scene_context_value(target_scene),
        );
    }

    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter_for_scene_generation(title: Option<&str>, summary: Option<&str>) -> chapter::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter::Model {
            id: 1,
            project_id: 10,
            outline_id: None,
            chapter_number: 3,
            title: title.map(str::to_string),
            content: "章节正文".to_string(),
            word_count: 4,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: summary.map(str::to_string),
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn scene_generation_input(scene_number: i32, scene_prompt: &str) -> GenerateChapterSceneInput {
        GenerateChapterSceneInput {
            chapter_id: 1,
            scene_number,
            scene_prompt: scene_prompt.to_string(),
        }
    }

    #[test]
    fn required_scene_generation_inputs_accepts_valid_input() {
        let chapter = chapter_for_scene_generation(Some("旧城"), Some("主角进入旧城"));
        let input = scene_generation_input(1, " 写入城门 ");

        let (title, summary, prompt) =
            required_scene_generation_inputs(&chapter, &input).expect("valid scene input");

        assert_eq!(title, "旧城");
        assert_eq!(summary, "主角进入旧城");
        assert_eq!(prompt, "写入城门");
    }

    #[test]
    fn required_scene_generation_inputs_rejects_missing_chapter_title() {
        let chapter = chapter_for_scene_generation(Some("  "), Some("主角进入旧城"));
        let input = scene_generation_input(1, "写入城门");

        let err = required_scene_generation_inputs(&chapter, &input)
            .expect_err("missing chapter title must fail");

        assert!(err.to_string().contains("已有标题"));
    }

    #[test]
    fn required_scene_generation_inputs_rejects_missing_chapter_summary() {
        let chapter = chapter_for_scene_generation(Some("旧城"), None);
        let input = scene_generation_input(1, "写入城门");

        let err = required_scene_generation_inputs(&chapter, &input)
            .expect_err("missing chapter summary must fail");

        assert!(err.to_string().contains("已有摘要"));
    }

    #[test]
    fn required_scene_generation_inputs_rejects_invalid_scene_number() {
        let chapter = chapter_for_scene_generation(Some("旧城"), Some("主角进入旧城"));
        let input = scene_generation_input(0, "写入城门");

        let err = required_scene_generation_inputs(&chapter, &input)
            .expect_err("invalid scene number must fail");

        assert!(err.to_string().contains("场景编号必须大于 0"));
    }

    #[test]
    fn required_scene_generation_inputs_rejects_blank_scene_prompt() {
        let chapter = chapter_for_scene_generation(Some("旧城"), Some("主角进入旧城"));
        let input = scene_generation_input(1, " \n\t ");

        let err = required_scene_generation_inputs(&chapter, &input)
            .expect_err("blank scene prompt must fail");

        assert!(err.to_string().contains("场景引导"));
    }

    #[test]
    fn trimmed_optional_text_normalizes_blank_to_none() {
        assert_eq!(trimmed_optional_text(" \n\t "), None);
        assert_eq!(trimmed_optional_text(" 旧城 "), Some("旧城".to_string()));
    }

    #[test]
    fn required_updated_text_rejects_blank_value() {
        let err = required_updated_text(" \n\t ", "场景正文")
            .expect_err("blank updated scene content must fail");

        assert!(matches!(err, AppError::InvalidArgument(message) if message == "场景正文不能为空"));
        assert_eq!(
            required_updated_text("  场景正文  ", "场景正文").expect("nonblank text"),
            "场景正文"
        );
    }

    #[test]
    fn build_scene_generation_user_prompt_uses_json_payload() {
        let prompt = build_scene_generation_user_prompt(
            2,
            "旧城",
            "主角进入旧城",
            Some(&json!({
                "chapter_outline": {
                    "key_scenes": ["城门盘查"]
                },
                "existing_scenes": [{
                    "scene_number": 0,
                    "content": "主角抵达城外。"
                }]
            })),
            1,
            "写入城门",
        )
        .expect("scene prompt should serialize");

        assert!(prompt.contains(r#""chapter_number": 2"#));
        assert!(prompt.contains(r#""title": "旧城""#));
        assert!(prompt.contains(r#""key_scenes""#));
        assert!(prompt.contains("主角抵达城外"));
        assert!(prompt.contains(r#""scene_number": 1"#));
        assert!(prompt.contains(r#""prompt": "写入城门""#));
        assert!(!prompt.contains("## 场景引导"));
    }

    #[test]
    fn build_scene_generation_user_prompt_rejects_blank_prompt() {
        let err = build_scene_generation_user_prompt(2, "旧城", "摘要", None, 1, " \n\t ")
            .expect_err("blank scene prompt must fail");

        assert!(err.to_string().contains("场景引导"));
    }

    #[test]
    fn normalize_scene_generation_output_rejects_blank() {
        let err = normalize_scene_generation_output(" \n\t ")
            .expect_err("blank generated scene must fail");

        assert!(err.to_string().contains("未返回"));
    }

    #[test]
    fn normalize_scene_generation_output_rejects_json() {
        let err =
            normalize_scene_generation_output(r#"{"title":"城门","content":"主角抵达城门。"}"#)
                .expect_err("scene generation must be plain text");

        assert!(err.to_string().contains("必须是纯文本正文"));
    }

    #[test]
    fn normalize_scene_generation_output_rejects_fenced_markdown() {
        let err = normalize_scene_generation_output("```text\n主角抵达城门。\n```")
            .expect_err("scene generation must not be fenced");

        assert!(err.to_string().contains("Markdown"));
    }

    #[test]
    fn restored_chapter_scene_checkpoint_revalidates_response_content() {
        let restored = restored_chapter_scene_checkpoint(
            json!({
                "output": "主角抵达城门，雨水沿着旧铜钉滑落。",
                "parsed_output": "主角抵达城门，雨水沿着旧铜钉滑落。",
                "response_content": "  主角抵达城门，雨水沿着旧铜钉滑落。  ",
            }),
            "chapter_scene_generation:3:1",
        )
        .expect("valid scene checkpoint should restore");

        assert_eq!(restored, "主角抵达城门，雨水沿着旧铜钉滑落。");
    }

    #[test]
    fn restored_chapter_scene_checkpoint_rejects_mismatched_output() {
        let err = restored_chapter_scene_checkpoint(
            json!({
                "output": "主角抵达城门。",
                "parsed_output": "主角抵达城门。",
                "response_content": "主角已经离开城门。",
            }),
            "chapter_scene_generation:3:1",
        )
        .expect_err("checkpoint output mismatch must fail");

        assert!(
            err.to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }
}
