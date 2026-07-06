use crate::AppState;
use crate::actions::dependency_conflict::{
    DownstreamConflictStrategy, delete_downstream_images_if_requested,
};
use crate::actions::manga_source::{
    checkpoint_matches_source, ensure_manga_source_can_write, resolve_chapter_manga_source,
    verify_manga_project_access, verify_manga_project_editor,
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_core::models::character::{character_portrait, protagonist_profile};
use a4n_core::models::project::novel_project;
use a4n_engine::llm::provider::{ChatRequest, LlmTraceContext, is_truncated_finish_reason};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_services::generation::context_aware_chat_request_trace_json;
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_services::manga::{MANGA_PAGE_PROMPT_TYPE, PIPELINE_STATUS_PROMPT_TYPE};
use a4n_services::source::source_content_sha256;
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

const MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME: &str = "manga_prompt_optimization";
const MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME: &str = "manga_prompt_optimization_user";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MangaPromptSummary {
    pub id: i64,
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: i32,
    pub panel_number: i32,
    pub prompt_type: String,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub characters_in_frame: Option<Value>,
    pub layout: Option<Value>,
    pub camera_angle: Option<String>,
    pub dialogue_position: Option<String>,
    pub generated_image_id: Option<i64>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStatusSummary {
    pub chapter_id: i64,
    pub pending: i64,
    pub generating: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub pipeline_status: Option<String>,
    pub stage: Option<String>,
    pub current: Option<i32>,
    pub total: Option<i32>,
    pub message: Option<String>,
    pub task_id: Option<String>,
    pub can_cancel: bool,
    pub can_resume: bool,
    pub has_checkpoint: bool,
    pub checkpoint_valid: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BindReferenceImageInput {
    pub prompt_id: i64,
    pub slot_id: String,
    pub asset_kind: String,
    pub asset_id: i64,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMangaPromptInput {
    pub prompt_id: i64,
    pub image_prompt: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeMangaPromptInput {
    pub prompt_id: i64,
    pub instruction: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptimizedMangaPromptFields {
    pub image_prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateMangaPromptInput {
    pub page_number: i32,
    #[serde(default)]
    pub panel_number: Option<i32>,
    pub image_prompt: String,
    #[serde(default)]
    pub dialogue_text: Option<String>,
    #[serde(default)]
    pub characters_in_frame: Option<Value>,
    #[serde(default)]
    pub layout: Option<Value>,
    #[serde(default)]
    pub camera_angle: Option<String>,
    #[serde(default)]
    pub dialogue_position: Option<String>,
}

pub async fn list_manga_panels(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<MangaPromptSummary>, String> {
    verify_manga_project_access(state, user_id, project_id)
        .await
        .map_err(|e| e.to_string())?;
    let panels = state
        .manga
        .list_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(panels.iter().map(to_summary).collect())
}

pub async fn create_manga_prompt(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: CreateMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    verify_manga_project_editor(state, user_id, project_id)
        .await
        .map_err(|e| e.to_string())?;
    let source = resolve_chapter_manga_source(state, chapter_id, project_id, false)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&source, "新增漫画提示词").map_err(|e| e.to_string())?;
    let page_number = validate_positive_i32(input.page_number, "页码")?;
    let panel_number = validate_non_negative_i32(input.panel_number.unwrap_or(0), "分镜序号")?;
    let image_prompt = required_trimmed(&input.image_prompt, "整页漫画提示词")?;
    let duplicate = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .filter(chapter_manga_prompt::Column::PageNumber.eq(page_number))
        .filter(chapter_manga_prompt::Column::PanelNumber.eq(panel_number))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    if duplicate.is_some() {
        return Err(format!(
            "第 {page_number} 页 / 分镜 {panel_number} 的漫画提示词已存在"
        ));
    }
    let now = chrono::Utc::now().naive_utc();
    let row = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(page_number),
        panel_number: Set(panel_number),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set(image_prompt.to_string()),
        dialogue_text: Set(normalize_optional_text(input.dialogue_text)),
        characters_in_frame: Set(input.characters_in_frame),
        layout: Set(input.layout),
        generated_image_id: Set(None),
        camera_angle: Set(normalize_optional_text(input.camera_angle)),
        dialogue_position: Set(normalize_optional_text(input.dialogue_position)),
        status: Set("pending".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(state.db.as_ref())
    .await
    .map_err(|e| e.to_string())?;
    Ok(to_summary(&row))
}

pub async fn bind_manga_reference_image(
    state: &AppState,
    user_id: i64,
    input: BindReferenceImageInput,
) -> Result<MangaPromptSummary, String> {
    let slot_id = required_trimmed(&input.slot_id, "参考图槽位")?;
    let panel = load_manga_page_prompt(state, input.prompt_id, "整页漫画提示词不存在").await?;
    verify_manga_project_editor(state, user_id, panel.project_id)
        .await
        .map_err(|e| e.to_string())?;
    let source = resolve_chapter_manga_source(state, panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&source, "绑定漫画参考图").map_err(|e| e.to_string())?;
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, input.downstream_strategy, "绑定参考图")?;

    let bound_asset = resolve_reference_bound_asset(
        state,
        panel.project_id,
        input.asset_kind.trim(),
        input.asset_id,
    )
    .await?;
    let layout = bind_reference_asset_in_layout(panel.layout.clone(), slot_id, bound_asset)?;

    let mut active: chapter_manga_prompt::ActiveModel = panel.into();
    active.layout = Set(Some(layout));
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    let updated = active
        .update(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
    Ok(to_summary(&updated))
}

pub async fn update_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: UpdateMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    let image_prompt = required_trimmed(&input.image_prompt, "整页漫画提示词")?;
    let panel = load_manga_page_prompt(state, input.prompt_id, "整页漫画提示词不存在").await?;
    verify_manga_project_editor(state, user_id, panel.project_id)
        .await
        .map_err(|e| e.to_string())?;
    let source = resolve_chapter_manga_source(state, panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&source, "修改整页漫画提示词").map_err(|e| e.to_string())?;

    if panel.image_prompt.trim() == image_prompt {
        return Ok(to_summary(&panel));
    }
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, input.downstream_strategy, "修改提示词")?;

    let mut active: chapter_manga_prompt::ActiveModel = panel.into();
    active.image_prompt = Set(image_prompt.to_string());
    active.status = Set("pending".to_string());
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    let updated = active
        .update(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
    Ok(to_summary(&updated))
}

pub async fn optimize_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: OptimizeMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    let instruction = required_trimmed(&input.instruction, "优化要求")?;
    let panel = load_manga_page_prompt(state, input.prompt_id, "整页漫画提示词不存在").await?;
    verify_manga_project_editor(state, user_id, panel.project_id)
        .await
        .map_err(|e| e.to_string())?;
    let source = resolve_chapter_manga_source(state, panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&source, "优化整页漫画提示词").map_err(|e| e.to_string())?;
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, input.downstream_strategy, "优化提示词")?;

    let mut request =
        build_manga_prompt_optimization_request(state.engine.as_ref(), &panel, instruction)
            .map_err(|e| e.to_string())?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let input_fingerprint_json = json!({
        "prompt_names": [
            MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
            MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        ],
        "request": request_json,
        "input_variables": {
            "instruction": instruction,
            "prompt_id": panel.id,
        },
        "source_refs": {
            "previous_prompt_id": panel.id,
            "previous_image_prompt": &panel.image_prompt,
            "dialogue_text": &panel.dialogue_text,
            "camera_angle": &panel.camera_angle,
            "dialogue_position": &panel.dialogue_position,
            "layout": &panel.layout,
            "characters_in_frame": &panel.characters_in_frame,
        },
    });
    let call_key = format!("optimize_manga_prompt:prompt:{}", panel.id);
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "manga_prompt_optimization:{}:{}",
                panel.project_id, panel.id
            ),
            workflow_kind: "manga_prompt_optimization".to_string(),
            project_id: Some(panel.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter_manga_prompt".to_string()),
            scope_id: Some(panel.id),
            chapter_id: Some(panel.chapter_id),
            mode: Some(panel.prompt_type.clone()),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .workflow_run
        .mark_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;

    let result = async {
        let (response_text, response_usage_json, response_finish_reason, restored) =
            if let Some(checkpoint) = load_completed_llm_call_checkpoint(
                state.db.clone(),
                workflow_run_id,
                &call_key,
                &input_fingerprint_json,
            )
            .await
            .map_err(|err| err.to_string())?
            {
                let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(|err| err.to_string())?;
                let (response_text, usage, finish_reason) =
                    manga_prompt_optimization_checkpoint_response(&checkpoint_json, &call_key)?;
                (response_text, usage, finish_reason, true)
            } else {
                let response = state
                    .llm
                    .chat(request)
                    .await
                    .map_err(|e| format!("优化漫画提示词失败: {e}"))?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err("优化漫画提示词输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“媒体提示词”输出档位，或缩短本次提示词后重试".to_string());
                }
                let response_text = response.message.content.clone();
                let response_usage_json = response
                    .usage
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("LLM 用量序列化失败: {err}"))?;
                let response_finish_reason = response.finish_reason.clone();
                (response_text, response_usage_json, response_finish_reason, false)
            };
        let optimized = parse_manga_prompt_optimization_output(&response_text)?;
        let parsed_image_prompt = optimized.image_prompt.clone();
        let image_prompt = required_trimmed(&optimized.image_prompt, "优化结果 image_prompt")?;
        if !restored {
            save_completed_llm_call_checkpoint(
                state.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key: call_key.clone(),
                    call_order: 1,
                    input_fingerprint_json: input_fingerprint_json.clone(),
                    checkpoint_json: json!({
                        "output": {
                            "image_prompt": image_prompt,
                        },
                        "parsed_output": {
                            "image_prompt": parsed_image_prompt,
                        },
                        "response_content": response_text,
                        "usage": response_usage_json,
                        "finish_reason": response_finish_reason,
                    }),
                    artifacts_json: None,
                },
            )
            .await
            .map_err(|err| err.to_string())?;
        }
        if panel.image_prompt.trim() == image_prompt {
            return Ok(panel.clone());
        }

        let mut active: chapter_manga_prompt::ActiveModel = panel.clone().into();
        active.image_prompt = Set(image_prompt.to_string());
        active.status = Set("pending".to_string());
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        let updated = active
            .update(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
        Ok(updated)
    }
    .await;

    if let Err(status_err) =
        finalize_manga_prompt_optimization_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "漫画提示词优化完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result.map(|row| to_summary(&row))
}

pub async fn delete_manga_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), String> {
    verify_manga_project_editor(state, user_id, project_id)
        .await
        .map_err(|e| e.to_string())?;
    let source = resolve_chapter_manga_source(state, chapter_id, project_id, false)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&source, "删除漫画产物").map_err(|e| e.to_string())?;
    state
        .manga
        .delete_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())
}

async fn finalize_manga_prompt_optimization_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<chapter_manga_prompt::Model, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .workflow_run
        .mark_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub async fn delete_manga_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let panel = load_manga_page_prompt(state, prompt_id, "整页漫画提示词不存在").await?;
    verify_manga_project_editor(state, user_id, panel.project_id)
        .await
        .map_err(|e| e.to_string())?;
    let source = resolve_chapter_manga_source(state, panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&source, "删除漫画提示词").map_err(|e| e.to_string())?;
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, downstream_strategy, "删除提示词")?;
    delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
    state
        .manga
        .delete_prompt(prompt_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_manga_pipeline_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<PipelineStatusSummary, String> {
    verify_manga_project_access(state, user_id, project_id)
        .await
        .map_err(|e| e.to_string())?;
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut pending: i64 = 0;
    let mut generating: i64 = 0;
    let mut completed: i64 = 0;
    let mut failed: i64 = 0;
    let mut cancelled: i64 = 0;
    for prompt in &prompts {
        match prompt.status.as_str() {
            "pending" => pending += 1,
            "generating" => generating += 1,
            "completed" => completed += 1,
            "failed" => failed += 1,
            "cancelled" => cancelled += 1,
            _ => {}
        }
    }

    let status_row = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(PIPELINE_STATUS_PROMPT_TYPE))
        .order_by_desc(chapter_manga_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_manga_prompt::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let layout = match status_row.as_ref() {
        Some(row) => Some(
            row.layout
                .as_ref()
                .ok_or_else(|| format!("漫画流水线状态 #{} 缺少 layout", row.id))?,
        ),
        None => None,
    };
    let pipeline_status = layout
        .map(|value| required_layout_string(value, "pipeline_status"))
        .transpose()?;
    let stage = layout
        .map(|value| required_layout_string(value, "stage"))
        .transpose()?;
    let current = layout
        .map(|value| required_layout_i32(value, "current"))
        .transpose()?;
    let total = layout
        .map(|value| required_layout_i32(value, "total"))
        .transpose()?;
    let message = layout
        .map(|value| required_layout_string(value, "message"))
        .transpose()?;
    let task_id = match layout {
        Some(value) => optional_layout_string(value, "task_id")?,
        None => None,
    };
    let has_checkpoint = match layout.and_then(|value| value.get("checkpoint_data")) {
        Some(value) => value
            .as_object()
            .map(|map| !map.is_empty())
            .ok_or_else(|| "漫画流水线 checkpoint_data 必须是对象".to_string())?,
        None => false,
    };
    let checkpoint_valid = if has_checkpoint {
        let source = resolve_chapter_manga_source(state, chapter_id, project_id, false)
            .await
            .map_err(|e| e.to_string())?;
        let layout =
            layout.ok_or_else(|| "漫画流水线断点缺少来源元数据所在的 layout".to_string())?;
        checkpoint_matches_source(layout, &source).map_err(|e| e.to_string())?
    } else {
        false
    };
    let can_cancel = match layout {
        Some(value) => required_layout_bool(value, "can_cancel")?,
        None => false,
    };
    let can_resume = match layout {
        Some(value) => required_layout_bool(value, "can_resume")?,
        None => false,
    };

    Ok(PipelineStatusSummary {
        chapter_id,
        pending,
        generating,
        completed,
        failed,
        cancelled,
        pipeline_status,
        stage,
        current,
        total,
        message,
        task_id,
        can_cancel,
        can_resume: can_resume && checkpoint_valid,
        has_checkpoint,
        checkpoint_valid,
    })
}

pub async fn cancel_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<i64, String> {
    verify_manga_project_editor(state, user_id, project_id)
        .await
        .map_err(|e| e.to_string())?;
    let source = resolve_chapter_manga_source(state, chapter_id, project_id, false)
        .await
        .map_err(|e| e.to_string())?;
    ensure_manga_source_can_write(&source, "取消漫画流水线").map_err(|e| e.to_string())?;
    state
        .manga
        .cancel_pipeline(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())
}

fn to_summary(p: &chapter_manga_prompt::Model) -> MangaPromptSummary {
    MangaPromptSummary {
        id: p.id,
        chapter_id: p.chapter_id,
        project_id: p.project_id,
        page_number: p.page_number,
        panel_number: p.panel_number,
        prompt_type: p.prompt_type.clone(),
        image_prompt: p.image_prompt.clone(),
        dialogue_text: p.dialogue_text.clone(),
        characters_in_frame: p.characters_in_frame.clone(),
        layout: p.layout.clone(),
        camera_angle: p.camera_angle.clone(),
        dialogue_position: p.dialogue_position.clone(),
        generated_image_id: p.generated_image_id,
        status: p.status.clone(),
        created_at: Some(p.created_at.to_string()),
        updated_at: Some(p.updated_at.to_string()),
    }
}

async fn load_manga_page_prompt(
    state: &AppState,
    prompt_id: i64,
    not_found_message: &str,
) -> Result<chapter_manga_prompt::Model, String> {
    let panel = chapter_manga_prompt::Entity::find_by_id(prompt_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| not_found_message.to_string())?;
    if panel.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err("只能操作整页漫画提示词".to_string());
    }
    Ok(panel)
}

fn resolve_manga_prompt_downstream_conflict(
    panel: &chapter_manga_prompt::Model,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Option<i64>, String> {
    let Some(image_id) = panel.generated_image_id else {
        return Ok(None);
    };
    match strategy {
        DownstreamConflictStrategy::Reject => Err(format!(
            "整页漫画提示词 #{} 已派生成图片 #{image_id}，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖源提示词",
            panel.id
        )),
        DownstreamConflictStrategy::KeepDownstream => Ok(None),
        DownstreamConflictStrategy::DeleteDownstream => Ok(Some(image_id)),
    }
}

async fn resolve_reference_bound_asset(
    state: &AppState,
    project_id: i64,
    asset_kind: &str,
    asset_id: i64,
) -> Result<Value, String> {
    let allowed_project_ids = allowed_reference_project_ids(state, project_id).await?;
    match asset_kind {
        "character_portrait" => {
            let portrait = character_portrait::Entity::find_by_id(asset_id)
                .one(state.db.as_ref())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("角色立绘 #{asset_id} 不存在"))?;
            if !allowed_project_ids.contains(&portrait.project_id) {
                return Err(format!(
                    "角色立绘 #{asset_id} 不属于当前漫画项目或根文本项目"
                ));
            }
            if portrait.image_path.trim().is_empty() {
                return Err(format!("角色立绘 #{asset_id} 缺少图片路径"));
            }
            let profile = protagonist_profile::Entity::find_by_id(portrait.profile_id)
                .one(state.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;
            let label = profile
                .as_ref()
                .map(|profile| profile.name.trim())
                .filter(|name| !name.is_empty())
                .unwrap_or("角色立绘");
            Ok(json!({
                "asset_kind": "character_portrait",
                "asset_id": portrait.id,
                "portrait_id": portrait.id,
                "profile_id": portrait.profile_id,
                "label": label,
                "image_path": portrait.image_path,
                "prompt": portrait.prompt,
                "portrait_type": portrait.portrait_type,
                "expression": portrait.expression,
                "pose": portrait.pose,
                "is_primary": portrait.is_primary,
            }))
        }
        "generated_image" => {
            let image = generated_image::Entity::find_by_id(asset_id)
                .one(state.db.as_ref())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("图片 #{asset_id} 不存在"))?;
            if !allowed_project_ids.contains(&image.project_id) {
                return Err(format!("图片 #{asset_id} 不属于当前漫画项目或根文本项目"));
            }
            if image.file_path.trim().is_empty() {
                return Err(format!("图片 #{asset_id} 缺少文件路径"));
            }
            Ok(json!({
                "asset_kind": "generated_image",
                "asset_id": image.id,
                "image_id": image.id,
                "label": format!("{} #{}", image.image_type, image.id),
                "image_type": image.image_type,
                "image_path": image.file_path,
                "prompt": image.prompt,
                "chapter_id": image.chapter_id,
                "panel_id": image.panel_id,
                "is_favorite": image.is_favorite,
            }))
        }
        other => Err(format!(
            "不支持的参考图素材类型: {other}，只能是 character_portrait 或 generated_image"
        )),
    }
}

async fn allowed_reference_project_ids(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<i64>, String> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;
    match project.project_kind.as_str() {
        "text" => Ok(vec![project.id]),
        "comic" => {
            let root_project_id = project
                .root_project_id
                .ok_or_else(|| format!("漫画项目缺少根文本项目: {}", project.id))?;
            Ok(vec![project.id, root_project_id])
        }
        other => Err(format!("项目 {project_id} 不是文本项目或漫画项目: {other}")),
    }
}

fn bind_reference_asset_in_layout(
    layout: Option<Value>,
    slot_id: &str,
    bound_asset: Value,
) -> Result<Value, String> {
    let mut layout = match layout {
        Some(Value::Object(map)) => map,
        Some(_) => return Err("整页漫画提示词 layout 必须是 JSON 对象".into()),
        None => serde_json::Map::new(),
    };
    let asset_kind = bound_asset
        .get("asset_kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "绑定参考图素材缺少 asset_kind".to_string())?;

    let mut bindings = match layout.remove("reference_bindings") {
        None => Vec::new(),
        Some(Value::Array(items)) => items,
        Some(_) => return Err("整页漫画提示词 reference_bindings 必须是数组".to_string()),
    };
    let mut found = false;
    for binding in &mut bindings {
        let Some(map) = binding.as_object_mut() else {
            continue;
        };
        let current_slot = map
            .get("slot_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if current_slot != slot_id {
            continue;
        }
        map.insert("status".into(), json!("bound"));
        map.insert("asset_kind".into(), json!(asset_kind));
        map.insert("bound_asset".into(), bound_asset.clone());
        found = true;
        break;
    }
    if !found {
        bindings.push(json!({
            "slot_id": slot_id,
            "label": slot_id,
            "target_type": "visual",
            "target_name": slot_id,
            "purpose": "生成整页漫画时使用绑定参考图",
            "required": true,
            "status": "bound",
            "asset_kind": asset_kind,
            "bound_asset": bound_asset,
        }));
    }

    layout.insert("reference_bindings".into(), Value::Array(bindings));
    Ok(Value::Object(layout))
}

fn required_layout_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是非空字符串"))
}

fn optional_layout_string(value: &Value, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty())
            .map(Some)
            .ok_or_else(|| format!("漫画流水线状态字段 {key} 必须是非空字符串或 null")),
    }
}

fn required_layout_i32(value: &Value, key: &str) -> Result<i32, String> {
    let raw = value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是整数"))?;
    i32::try_from(raw).map_err(|_| format!("漫画流水线状态字段 {key} 超出 i32 范围"))
}

fn required_layout_bool(value: &Value, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是布尔值"))
}

fn build_manga_prompt_optimization_request(
    prompt_engine: &dyn PromptEngine,
    prompt: &chapter_manga_prompt::Model,
    instruction: &str,
) -> Result<ChatRequest, AppError> {
    let variables = HashMap::new();
    let system_prompt = prompt_engine.render(
        MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
        &variables,
        Some(prompt.project_id),
    )?;
    let dynamic_payload = json!({
        "context_refs": {
            "current_prompt": "稳定上下文包 source_artifact.current_prompt",
        },
        "instruction": instruction,
        "output_contract": {
            "format": "json_object",
            "required_fields": ["image_prompt"],
            "field_rules": {
                "image_prompt": "优化后的整页漫画绘图提示词，不能为空"
            }
        }
    });
    let mut user_variables = HashMap::new();
    user_variables.insert(
        "payload".to_string(),
        json!(serde_json::to_string_pretty(&dynamic_payload)?),
    );
    let user_prompt = prompt_engine.render(
        MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        &user_variables,
        Some(prompt.project_id),
    )?;
    let current_prompt = manga_prompt_optimization_source_artifact(prompt);
    let source_hash = source_content_sha256(
        &serde_json::to_string(&current_prompt).map_err(AppError::Serialization)?,
    );
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(a4n_services::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("prompt_optimization").into(),
        pack_kind: ("source_artifact").into(),
        source_refs_json: Some(json!({
            "source": "chapter_manga_prompt",
            "prompt_id": prompt.id,
            "content_sha256": source_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "manga_prompt_optimization_context_v1",
            "artifact_kind": "chapter_manga_prompt",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, current_prompt.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, current_prompt.clone()),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "artifact_kind": "chapter_manga_prompt",
                    "prompt_id": prompt.id,
                    "page_number": prompt.page_number,
                    "panel_number": prompt.panel_number,
                    "prompt_type": prompt.prompt_type,
                    "image_prompt": prompt.image_prompt,
                }),
            ),
        ],
    })?;
    let manifest = ledger.into_manifest(
        user_prompt,
        Some("漫画提示词优化必须保留稳定上下文包 source_artifact 中的剧情、角色、构图、参考图和风格约束，只按动态输入中的 instruction 修改表达。".to_string()),
    );
    let fingerprint = source_content_sha256(
        &serde_json::to_string(&json!({
            "prompt_id": prompt.id,
            "source_artifact_sha256": source_hash,
            "instruction": instruction,
            "prompt_names": [
                MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
                MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
            ],
        }))
        .map_err(AppError::Serialization)?,
    );
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.25),
        None,
        Some(llm_token_profiles::MEDIA_PROMPT.to_string()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(prompt.project_id),
        root_project_id: None,
        workflow: "manga_prompt_optimization".to_string(),
        step_key: "optimize_manga_prompt".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(format!("optimize_manga_prompt:prompt:{}", prompt.id)),
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn manga_prompt_optimization_source_artifact(prompt: &chapter_manga_prompt::Model) -> Value {
    json!({
        "artifact_kind": "chapter_manga_prompt",
        "current_prompt": {
            "prompt_id": prompt.id,
            "page_number": prompt.page_number,
            "panel_number": prompt.panel_number,
            "prompt_type": prompt.prompt_type,
            "dialogue_text": prompt.dialogue_text,
            "camera_angle": prompt.camera_angle,
            "dialogue_position": prompt.dialogue_position,
            "image_prompt": prompt.image_prompt
        }
    })
}

fn parse_manga_prompt_optimization_output(raw: &str) -> Result<OptimizedMangaPromptFields, String> {
    let value = crate::actions::json_contract::parse_llm_json_object(raw, "漫画提示词优化结果")?;
    serde_json::from_value(value).map_err(|e| format!("漫画提示词优化结果字段不符合要求: {e}"))
}

fn manga_prompt_optimization_checkpoint_response(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(String, Option<Value>, Option<String>), String> {
    let response_text = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed = parse_manga_prompt_optimization_output(&response_text)?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    let checkpoint_parsed: OptimizedMangaPromptFields =
        serde_json::from_value(parsed_output.clone())
            .map_err(|err| format!("{call_key}.parsed_output 字段不符合要求: {err}"))?;
    if checkpoint_parsed.image_prompt != parsed.image_prompt {
        return Err(format!(
            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
        ));
    }
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{call_key}.finish_reason 必须是字符串或 null"))?,
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        ));
    }
    Ok((response_text, usage, finish_reason))
}

fn required_trimmed<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    Ok(trimmed)
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_positive_i32(value: i32, label: &str) -> Result<i32, String> {
    if value <= 0 {
        return Err(format!("{label}必须大于 0"));
    }
    Ok(value)
}

fn validate_non_negative_i32(value: i32, label: &str) -> Result<i32, String> {
    if value < 0 {
        return Err(format!("{label}不能小于 0"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        OptimizeMangaPromptInput, UpdateMangaPromptInput, bind_reference_asset_in_layout,
        optional_layout_string, required_layout_bool, required_layout_i32, required_layout_string,
    };
    use serde_json::{Value, json};

    #[test]
    fn bind_reference_asset_updates_existing_slot() {
        let layout = json!({
            "reference_bindings": [
                {"slot_id": "hero", "status": "missing"}
            ]
        });
        let bound = json!({"asset_kind": "generated_image", "image_id": 7});

        let updated = bind_reference_asset_in_layout(Some(layout), "hero", bound)
            .expect("existing slot should be updated");
        let binding = updated["reference_bindings"][0]
            .as_object()
            .expect("binding should be object");
        assert_eq!(binding.get("status"), Some(&json!("bound")));
        assert_eq!(binding.get("asset_kind"), Some(&json!("generated_image")));
        assert_eq!(
            binding.get("bound_asset"),
            Some(&json!({
                "asset_kind": "generated_image",
                "image_id": 7,
            }))
        );
    }

    #[test]
    fn bind_reference_asset_appends_missing_slot() {
        let bound = json!({"asset_kind": "character_portrait", "portrait_id": 9});
        let updated = bind_reference_asset_in_layout(None, "hero", bound)
            .expect("missing layout should be created");
        let bindings = updated["reference_bindings"]
            .as_array()
            .expect("bindings should be array");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0]["slot_id"], json!("hero"));
        assert_eq!(bindings[0]["status"], json!("bound"));
    }

    #[test]
    fn bind_reference_asset_rejects_non_object_layout() {
        let err = bind_reference_asset_in_layout(Some(Value::Array(Vec::new())), "hero", json!({}))
            .expect_err("non object layout should fail");
        assert_eq!(err, "整页漫画提示词 layout 必须是 JSON 对象");
    }

    #[test]
    fn bind_reference_asset_rejects_non_array_reference_bindings() {
        let layout = json!({"reference_bindings": {"slot_id": "hero"}});
        let err = bind_reference_asset_in_layout(
            Some(layout),
            "hero",
            json!({"asset_kind": "generated_image"}),
        )
        .expect_err("non-array reference_bindings should fail");
        assert_eq!(err, "整页漫画提示词 reference_bindings 必须是数组");
    }

    #[test]
    fn bind_reference_asset_rejects_missing_asset_kind() {
        let err = bind_reference_asset_in_layout(None, "hero", json!({"image_id": 1}))
            .expect_err("bound asset kind must be explicit");
        assert_eq!(err, "绑定参考图素材缺少 asset_kind");
    }

    #[test]
    fn pipeline_layout_fields_validate_types() {
        let layout = json!({
            "pipeline_status": "running",
            "current": 2,
            "can_cancel": true,
            "task_id": null,
        });
        assert_eq!(
            required_layout_string(&layout, "pipeline_status").unwrap(),
            "running"
        );
        assert_eq!(required_layout_i32(&layout, "current").unwrap(), 2);
        assert!(required_layout_bool(&layout, "can_cancel").unwrap());
        assert_eq!(optional_layout_string(&layout, "task_id").unwrap(), None);
    }

    #[test]
    fn manga_prompt_optimization_output_parses_json() {
        let parsed = super::parse_manga_prompt_optimization_output(
            r#"{"image_prompt":"优化后的漫画整页提示词"}"#,
        )
        .expect("json object should parse");
        assert_eq!(parsed.image_prompt, "优化后的漫画整页提示词");
    }

    #[test]
    fn manga_prompt_inputs_reject_unknown_fields() {
        let update_err = serde_json::from_value::<UpdateMangaPromptInput>(json!({
            "prompt_id": 1,
            "image_prompt": "新的漫画提示词",
            "legacy_notes": "旧字段"
        }))
        .expect_err("unknown update input fields should fail");
        assert!(update_err.to_string().contains("unknown field"));

        let optimize_err = serde_json::from_value::<OptimizeMangaPromptInput>(json!({
            "prompt_id": 1,
            "instruction": "增强构图",
            "legacy_notes": "旧字段"
        }))
        .expect_err("unknown optimize input fields should fail");
        assert!(optimize_err.to_string().contains("unknown field"));
    }

    #[test]
    fn manga_prompt_optimization_output_rejects_fenced_json() {
        let err = super::parse_manga_prompt_optimization_output(
            "```json\n{\"image_prompt\":\"优化后的漫画整页提示词\"}\n```",
        )
        .expect_err("fenced json must fail instead of being silently unwrapped");
        assert!(err.contains("JSON"));
    }

    #[test]
    fn manga_prompt_optimization_output_rejects_unknown_fields() {
        let err = super::parse_manga_prompt_optimization_output(
            r#"{"image_prompt":"优化后的漫画整页提示词","legacy_notes":"旧字段"}"#,
        )
        .expect_err("unknown optimized fields should fail");

        assert!(err.contains("unknown field"));
    }

    #[test]
    fn manga_prompt_optimization_checkpoint_revalidates_response_content() {
        let checkpoint = json!({
            "response_content": r#"{"image_prompt":"优化后的漫画整页提示词"}"#,
            "parsed_output": {
                "image_prompt": "优化后的漫画整页提示词"
            },
            "usage": null,
            "finish_reason": "stop",
        });

        let (response_text, usage, finish_reason) =
            super::manga_prompt_optimization_checkpoint_response(
                &checkpoint,
                "optimize_manga_prompt:prompt:1",
            )
            .expect("valid checkpoint should restore");

        assert_eq!(
            response_text,
            r#"{"image_prompt":"优化后的漫画整页提示词"}"#
        );
        assert_eq!(usage, None);
        assert_eq!(finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn manga_prompt_optimization_checkpoint_rejects_mismatched_parsed_output() {
        let checkpoint = json!({
            "response_content": r#"{"image_prompt":"响应里的提示词"}"#,
            "parsed_output": {
                "image_prompt": "断点里的提示词"
            },
            "usage": null,
            "finish_reason": "stop",
        });

        let err = super::manga_prompt_optimization_checkpoint_response(
            &checkpoint,
            "optimize_manga_prompt:prompt:1",
        )
        .expect_err("checkpoint parsed output must match reparsed response");

        assert!(err.contains("重新解析结果不一致"));
    }
}
