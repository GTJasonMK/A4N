use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_core::state::status::ProjectStatus;
use a4n_engine::llm::provider::ChatRequest;
use a4n_services::outline::{
    BeatMappingApplyInput, ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput,
    ChapterOutlineRegenerationInput, OutlineRequestPreviewStep, PartOutlineContinuationInput,
    PartOutlineGenerationInput, PartOutlineRegenerationInput,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize, de};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub type OutlineProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

async fn start_outline_workflow_run(
    state: &AppState,
    project_id: i64,
    workflow_kind: &str,
    mode: &str,
    current_step_key: &str,
    task_id: Option<String>,
    input_fingerprint_json: Value,
) -> Result<i64, AppError> {
    let run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("{workflow_kind}:project:{project_id}:{mode}"),
            workflow_kind: workflow_kind.to_string(),
            project_id: Some(project_id),
            root_project_id: Some(project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(input_fingerprint_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(current_step_key.to_string()),
            active_task_id: task_id,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    Ok(run.id)
}

async fn complete_outline_workflow_run(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<(), AppError> {
    state
        .workflow_run
        .mark_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await?;
    Ok(())
}

async fn fail_outline_workflow_run(state: &AppState, workflow_run_id: i64, err: &AppError) {
    let _ = state
        .workflow_run
        .mark_status(
            workflow_run_id,
            WORKFLOW_STATUS_FAILED,
            true,
            Some(&err.to_string()),
        )
        .await;
}

async fn unwrap_outline_workflow_result<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            fail_outline_workflow_run(state, workflow_run_id, &err).await;
            Err(err)
        }
    }
}

async fn finish_outline_postprocess(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<(), AppError>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => complete_outline_workflow_run(state, workflow_run_id).await,
        Err(err) => {
            fail_outline_workflow_run(state, workflow_run_id, &err).await;
            Err(err)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartOutlineSummary {
    pub id: i64,
    pub project_id: i64,
    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub summary: Option<String>,
    pub theme: Option<String>,
    pub key_events: Option<Value>,
    pub character_focus: Option<Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationProgressSummary {
    pub total_parts: u64,
    pub completed_parts: u64,
    pub total_chapter_outlines: u64,
    pub completed_chapter_outlines: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterOutlineSummary {
    pub id: i64,
    pub project_id: i64,
    pub part_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: Option<i32>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct OutlineTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratePartsInput {
    pub project_id: i64,
    pub part_count: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegeneratePartsInput {
    pub project_id: i64,
    pub part_count: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegenerateLastPartInput {
    pub project_id: i64,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuePartOutlinesInput {
    pub project_id: i64,
    pub part_number: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterOutlineInput {
    pub project_id: i64,
    pub outline_id: i64,
    #[serde(default, deserialize_with = "deserialize_optional_outline_title")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_summary")]
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_beats")]
    pub beats: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_key_scenes")]
    pub key_scenes: Option<Value>,
}

fn deserialize_optional_outline_title<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_optional_outline_text(deserializer, "title")
}

fn deserialize_optional_outline_summary<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_optional_outline_text(deserializer, "summary")
}

fn deserialize_optional_outline_text<'de, D>(
    deserializer: D,
    field: &str,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(de::Error::custom(format!("章节大纲字段 {field} 不能为空")));
            }
            Ok(Some(trimmed.to_string()))
        }
        Value::Null => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 不能为 null"
        ))),
        _ => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 必须是字符串"
        ))),
    }
}

fn deserialize_optional_outline_beats<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_optional_outline_array(deserializer, "beats")
}

fn deserialize_optional_outline_key_scenes<'de, D>(
    deserializer: D,
) -> Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_optional_outline_array(deserializer, "key_scenes")
}

fn deserialize_optional_outline_array<'de, D>(
    deserializer: D,
    field: &str,
) -> Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Array(_) => Ok(Some(value)),
        Value::Null => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 不能为 null"
        ))),
        _ => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 必须是数组"
        ))),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegenerateChapterOutlineInput {
    pub project_id: i64,
    pub outline_id: i64,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegeneratePartOutlineInput {
    pub project_id: i64,
    pub part_id: i64,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratePartChapterOutlinesInput {
    pub project_id: i64,
    pub part_id: i64,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchGeneratePartOutlinesInput {
    pub project_id: i64,
    pub part_ids: Vec<i64>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyBeatMappingInput {
    pub project_id: i64,
    pub part_id: i64,
    pub beat_sheet: String,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterOutlinesByCountInput {
    pub project_id: i64,
    pub start_chapter: i32,
    pub count: i32,
    pub task_id: Option<String>,
}

pub async fn generate_parts_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GeneratePartsInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn regenerate_parts_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegeneratePartsInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn regenerate_last_part_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegenerateLastPartInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn continue_part_outlines_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ContinuePartOutlinesInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn regenerate_part_outline_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegeneratePartOutlineInput,
) -> Result<OutlineTaskScope, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: 3,
    })
}

pub async fn generate_part_chapter_outlines_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GeneratePartChapterOutlinesInput,
) -> Result<OutlineTaskScope, AppError> {
    let part = ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: progress_total(part.end_chapter - part.start_chapter + 1),
    })
}

pub async fn batch_generate_part_outlines_task_scope(
    state: &AppState,
    user_id: i64,
    input: &BatchGeneratePartOutlinesInput,
) -> Result<OutlineTaskScope, AppError> {
    let parts = collect_unique_parts(state, user_id, input.project_id, &input.part_ids).await?;
    let total = parts
        .iter()
        .map(|part| (part.end_chapter - part.start_chapter + 1).max(0) as u32)
        .sum::<u32>()
        .max(1);
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total,
    })
}

pub async fn apply_beat_mapping_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ApplyBeatMappingInput,
) -> Result<OutlineTaskScope, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: 3,
    })
}

pub async fn generate_chapter_outlines_by_count_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterOutlinesByCountInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(
        state,
        user_id,
        input.project_id,
        progress_total(input.count),
    )
    .await
}

pub async fn regenerate_chapter_outline_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegenerateChapterOutlineInput,
) -> Result<OutlineTaskScope, AppError> {
    ensure_chapter_outline_access(state, user_id, input.project_id, input.outline_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: 3,
    })
}

fn part_outline_summary(part: &part_outline::Model) -> PartOutlineSummary {
    PartOutlineSummary {
        id: part.id,
        project_id: part.project_id,
        part_number: part.part_number,
        title: part.title.clone(),
        start_chapter: part.start_chapter,
        end_chapter: part.end_chapter,
        summary: part.summary.clone(),
        theme: part.theme.clone(),
        key_events: part.key_events.clone(),
        character_focus: part.character_focus.clone(),
        created_at: Some(part.created_at.to_string()),
        updated_at: Some(part.updated_at.to_string()),
    }
}

fn chapter_outline_summary(outline: &chapter_outline::Model) -> ChapterOutlineSummary {
    ChapterOutlineSummary {
        id: outline.id,
        project_id: outline.project_id,
        part_id: outline.part_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        beats: outline.beats.clone(),
        key_scenes: outline.key_scenes.clone(),
        pacing_score: outline.pacing_score,
        foreshadowing_setup: outline.foreshadowing_setup.clone(),
        foreshadowing_payoff: outline.foreshadowing_payoff.clone(),
        characters_involved: outline.characters_involved.clone(),
        word_count_target: outline.word_count_target,
        status: outline.status.clone(),
        created_at: Some(outline.created_at.to_string()),
        updated_at: Some(outline.updated_at.to_string()),
    }
}

pub async fn list_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    let parts = state.outline.list_parts(project_id).await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn generate_parts(
    state: &AppState,
    user_id: i64,
    input: GeneratePartsInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    report(progress, 0, 3, "启动分部大纲生成")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "generate",
        "part_1",
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "generate",
            "project_id": input.project_id,
            "part_count": input.part_count,
            "chapters_per_part": input.chapters_per_part,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .outline
        .generate_parts(PartOutlineGenerationInput {
            project_id: input.project_id,
            part_count: input.part_count,
            chapters_per_part: input.chapters_per_part,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let parts = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "分部大纲已生成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn preview_generate_parts_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePartsInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let steps = state
        .outline
        .preview_generate_parts_requests(
            input.project_id,
            input.part_count,
            input.chapters_per_part,
            false,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    outline_preview(
        state,
        "part_outline_generation",
        "分部大纲生成请求预览",
        steps,
    )
    .await
}

pub async fn delete_latest_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<(), AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    state.outline.delete_latest_parts(project_id, count).await
}

pub async fn regenerate_parts(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartsInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    report(progress, 0, 3, "启动分部大纲重生成")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "regenerate_all",
        "part_1",
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "regenerate_all",
            "project_id": input.project_id,
            "part_count": input.part_count,
            "chapters_per_part": input.chapters_per_part,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .outline
        .regenerate_parts(PartOutlineRegenerationInput {
            project_id: input.project_id,
            part_count: input.part_count,
            chapters_per_part: input.chapters_per_part,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let parts = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "已重新生成所有部")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn preview_regenerate_parts_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartsInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let steps = state
        .outline
        .preview_generate_parts_requests(
            input.project_id,
            input.part_count,
            input.chapters_per_part,
            true,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    outline_preview(
        state,
        "part_outline_regeneration",
        "分部大纲重生成请求预览",
        steps,
    )
    .await
}

pub async fn regenerate_last_part(
    state: &AppState,
    user_id: i64,
    input: RegenerateLastPartInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<PartOutlineSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    report(progress, 0, 3, "正在重新生成最后一部")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "regenerate_last",
        "part_latest",
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "regenerate_last",
            "project_id": input.project_id,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .outline
        .regenerate_last_part(
            input.project_id,
            state.llm.as_ref(),
            state.engine.as_ref(),
            &variables,
            Some(workflow_run_id),
        )
        .await;
    let part = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            report(progress, 2, 3, "正在刷新项目状态")?;
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "最后一部已重新生成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(part_outline_summary(&part))
}

pub async fn preview_regenerate_last_part_request(
    state: &AppState,
    user_id: i64,
    input: RegenerateLastPartInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .outline
        .preview_regenerate_last_part_request(input.project_id, state.engine.as_ref(), &variables)
        .await?;
    single_request_preview(
        state,
        "part_outline_regenerate_last",
        "最后一部分部大纲重生成请求预览",
        "regenerate_last_part",
        "重新生成最后一部",
        "part_outline_single",
        request,
    )
    .await
}

pub async fn get_generation_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<GenerationProgressSummary, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(state.db.as_ref())
        .await?;
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id)
        .all(state.db.as_ref())
        .await?;
    let completed_parts = completed_part_count(&parts, &outlines)?;
    let completed_chapter_outlines = outlines
        .iter()
        .filter(|outline| outline.status == chapter_outline::STATUS_COMPLETED)
        .count() as u64;

    Ok(GenerationProgressSummary {
        total_parts: parts.len() as u64,
        completed_parts,
        total_chapter_outlines: outlines.len() as u64,
        completed_chapter_outlines,
    })
}

fn completed_part_count(
    parts: &[part_outline::Model],
    outlines: &[chapter_outline::Model],
) -> Result<u64, AppError> {
    let mut outlines_by_chapter = HashMap::<i32, &chapter_outline::Model>::new();
    for outline in outlines {
        if !chapter_outline::is_valid_status(&outline.status) {
            return Err(progress_state_error(format!(
                "章节大纲 {} 状态无效：{}",
                outline.id, outline.status
            )));
        }
        if outlines_by_chapter
            .insert(outline.chapter_number, outline)
            .is_some()
        {
            return Err(progress_state_error(format!(
                "章节号 {} 存在重复章节大纲",
                outline.chapter_number
            )));
        }
    }

    let mut claimed_chapters = HashSet::<i32>::new();
    let mut completed = 0_u64;
    for part in parts {
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(progress_state_error(format!(
                "分部 {} 章节范围无效：{}..={}",
                part.id, part.start_chapter, part.end_chapter
            )));
        }
        for chapter_number in part.start_chapter..=part.end_chapter {
            if !claimed_chapters.insert(chapter_number) {
                return Err(progress_state_error(format!(
                    "章节号 {} 被多个分部范围覆盖",
                    chapter_number
                )));
            }
        }

        let part_complete = (part.start_chapter..=part.end_chapter).all(|chapter_number| {
            outlines_by_chapter
                .get(&chapter_number)
                .is_some_and(|outline| outline.status == chapter_outline::STATUS_COMPLETED)
        });
        if part_complete {
            completed += 1;
        }
    }

    Ok(completed)
}

fn progress_state_error(current: String) -> AppError {
    AppError::InvalidState {
        current,
        operation: "查询大纲生成进度".to_string(),
    }
}

pub async fn continue_part_outlines(
    state: &AppState,
    user_id: i64,
    input: ContinuePartOutlinesInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<PartOutlineSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    report(
        progress,
        0,
        3,
        &format!("正在续作第 {} 部", input.part_number),
    )?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "continue",
        &format!("part_{}", input.part_number),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "continue",
            "project_id": input.project_id,
            "part_number": input.part_number,
            "start_chapter": input.start_chapter,
            "end_chapter": input.end_chapter,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .outline
        .continue_part_outline(PartOutlineContinuationInput {
            project_id: input.project_id,
            part_number: input.part_number,
            start_chapter: input.start_chapter,
            end_chapter: input.end_chapter,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let part = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            report(progress, 2, 3, "正在刷新项目状态")?;
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "分部续作完成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(part_outline_summary(&part))
}

pub async fn preview_continue_part_outlines_request(
    state: &AppState,
    user_id: i64,
    input: ContinuePartOutlinesInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .outline
        .preview_continue_part_outline_request(
            input.project_id,
            input.part_number,
            input.start_chapter,
            input.end_chapter,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    single_request_preview(
        state,
        "part_outline_continue",
        format!("第 {} 部续作请求预览", input.part_number),
        "continue_part_outline",
        format!("续作第 {} 部", input.part_number),
        "part_outline_single",
        request,
    )
    .await
}

pub async fn regenerate_part_outline(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartOutlineInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<PartOutlineSummary, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    report(progress, 0, 3, "正在重新生成指定分部")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "regenerate_one",
        &format!("part_id_{}", input.part_id),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "regenerate_one",
            "project_id": input.project_id,
            "part_id": input.part_id,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .outline
        .regenerate_part_outline(
            input.project_id,
            input.part_id,
            state.llm.as_ref(),
            state.engine.as_ref(),
            &variables,
            Some(workflow_run_id),
        )
        .await;
    let part = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            report(progress, 2, 3, "正在刷新项目状态")?;
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "指定分部已重新生成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(part_outline_summary(&part))
}

pub async fn preview_regenerate_part_outline_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartOutlineInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .outline
        .preview_regenerate_part_outline_request(
            input.project_id,
            input.part_id,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    single_request_preview(
        state,
        "part_outline_regenerate_one",
        "指定分部大纲重生成请求预览",
        "regenerate_part_outline",
        format!("重新生成分部 #{}", input.part_id),
        "part_outline_single",
        request,
    )
    .await
}

pub async fn list_chapter_outlines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    let outlines = state.outline.list_chapter_outlines(project_id).await?;
    Ok(outlines.iter().map(chapter_outline_summary).collect())
}

pub async fn delete_latest_chapter_outlines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<(), AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    crate::workflow::delete_latest_chapter_outlines_cascade(state, project_id, count)
        .await
        .map_err(AppError::InvalidArgument)
}

pub async fn update_chapter_outline(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterOutlineInput,
) -> Result<ChapterOutlineSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let updated = update_chapter_outline_record(state.db.as_ref(), input).await?;
    Ok(chapter_outline_summary(&updated))
}

pub async fn regenerate_chapter_outline(
    state: &AppState,
    user_id: i64,
    input: RegenerateChapterOutlineInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<ChapterOutlineSummary, AppError> {
    ensure_chapter_outline_access(state, user_id, input.project_id, input.outline_id).await?;
    report(progress, 0, 3, "启动章节大纲重生成")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "chapter_outline_generation",
        "regenerate_one",
        &format!("outline_id_{}", input.outline_id),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "chapter_outline_generation",
            "mode": "regenerate_one",
            "project_id": input.project_id,
            "outline_id": input.outline_id,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .outline
        .regenerate_chapter_outline(ChapterOutlineRegenerationInput {
            project_id: input.project_id,
            outline_id: input.outline_id,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let outline = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            advance_status(state, input.project_id, ProjectStatus::ChapterOutlinesReady).await?;
            report(progress, 3, 3, "章节大纲已重新生成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(chapter_outline_summary(&outline))
}

pub async fn preview_regenerate_chapter_outline_request(
    state: &AppState,
    user_id: i64,
    input: RegenerateChapterOutlineInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_chapter_outline_access(state, user_id, input.project_id, input.outline_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .outline
        .preview_regenerate_chapter_outline_request(
            input.project_id,
            input.outline_id,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    single_request_preview(
        state,
        "chapter_outline_regenerate_one",
        "章节大纲重生成请求预览",
        "regenerate_chapter_outline",
        format!("重新生成章节大纲 #{}", input.outline_id),
        "outline",
        request,
    )
    .await
}

pub async fn generate_part_chapter_outlines(
    state: &AppState,
    user_id: i64,
    input: GeneratePartChapterOutlinesInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    let part = ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    let count = part.end_chapter - part.start_chapter + 1;
    report(
        progress,
        0,
        progress_total(count),
        &format!("启动第 {} 部章节大纲生成", part.part_number),
    )?;
    let variables = HashMap::new();
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "chapter_outline_generation",
        "generate_part",
        &format!("chapter_{}", part.start_chapter),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "chapter_outline_generation",
            "mode": "generate_part",
            "project_id": input.project_id,
            "part_id": input.part_id,
            "start_chapter": part.start_chapter,
            "count": count,
        }),
    )
    .await?;
    let result = state
        .outline
        .generate_chapter_outlines(ChapterOutlineGenerationInput {
            project_id: input.project_id,
            part_id: part.id,
            start_chapter: part.start_chapter,
            count,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let outlines = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        advance_status(state, input.project_id, ProjectStatus::ChapterOutlinesReady).await,
    )
    .await?;
    Ok(outlines.iter().map(chapter_outline_summary).collect())
}

pub async fn preview_generate_part_chapter_outlines_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePartChapterOutlinesInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    let variables = HashMap::new();
    let steps = state
        .outline
        .preview_generate_part_chapter_outline_requests(
            input.project_id,
            input.part_id,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    outline_preview(
        state,
        "part_chapter_outline_generation",
        "本部章节大纲生成请求预览",
        steps,
    )
    .await
}

pub async fn batch_generate_part_outlines(
    state: &AppState,
    user_id: i64,
    input: BatchGeneratePartOutlinesInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    let parts = collect_unique_parts(state, user_id, input.project_id, &input.part_ids).await?;
    let total = parts
        .iter()
        .map(|part| (part.end_chapter - part.start_chapter + 1).max(0) as u32)
        .sum::<u32>()
        .max(1);
    report(
        progress,
        0,
        total,
        &format!("启动 {} 个分部的章节大纲生成", parts.len()),
    )?;

    let variables = HashMap::new();
    let part_fingerprint_json = parts
        .iter()
        .map(|part| {
            json!({
                "part_id": part.id,
                "part_number": part.part_number,
                "start_chapter": part.start_chapter,
                "end_chapter": part.end_chapter,
            })
        })
        .collect::<Vec<_>>();
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "chapter_outline_generation",
        "generate_batch_parts",
        "chapter_batch",
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "chapter_outline_generation",
            "mode": "generate_batch_parts",
            "project_id": input.project_id,
            "parts": part_fingerprint_json,
        }),
    )
    .await?;
    let mut results = Vec::new();
    let mut completed = 0_u32;
    let batch_result = async {
        for part in parts {
            let count = part.end_chapter - part.start_chapter + 1;
            let offset_progress = |current: u32, _total: u32, message: &str| {
                report(progress, completed + current, total, message)
            };
            let outlines = state
                .outline
                .generate_chapter_outlines(ChapterOutlineGenerationInput {
                    project_id: input.project_id,
                    part_id: part.id,
                    start_chapter: part.start_chapter,
                    count,
                    llm: state.llm.as_ref(),
                    prompt_engine: state.engine.as_ref(),
                    variables: &variables,
                    progress: Some(&offset_progress),
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            completed += count.max(0) as u32;
            results.extend(outlines.iter().map(chapter_outline_summary));
        }
        Ok::<(), AppError>(())
    }
    .await;
    unwrap_outline_workflow_result(state, workflow_run_id, batch_result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        advance_status(state, input.project_id, ProjectStatus::ChapterOutlinesReady).await,
    )
    .await?;
    Ok(results)
}

pub async fn preview_batch_generate_part_outlines_request(
    state: &AppState,
    user_id: i64,
    input: BatchGeneratePartOutlinesInput,
) -> Result<ModelRequestPreview, AppError> {
    let parts = collect_unique_parts(state, user_id, input.project_id, &input.part_ids).await?;
    let variables = HashMap::new();
    let mut all_steps = Vec::new();
    let mut first_ready_seen = false;
    for part in parts {
        let mut steps = state
            .outline
            .preview_generate_part_chapter_outline_requests(
                input.project_id,
                part.id,
                state.engine.as_ref(),
                &variables,
            )
            .await?;
        for step in &mut steps {
            if step.state == "ready" {
                if first_ready_seen {
                    step.state = "blocked".to_string();
                    step.note = Some(
                        "该请求位于批量流程后续阶段，依赖前序章节大纲生成并写入后的上下文。"
                            .to_string(),
                    );
                    step.request = None;
                } else {
                    first_ready_seen = true;
                }
            }
        }
        all_steps.extend(steps);
    }
    outline_preview(
        state,
        "batch_part_chapter_outline_generation",
        "批量章节大纲生成请求预览",
        all_steps,
    )
    .await
}

pub async fn apply_beat_mapping(
    state: &AppState,
    user_id: i64,
    input: ApplyBeatMappingInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    report(progress, 0, 3, "启动节拍映射")?;
    let variables = HashMap::new();
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "chapter_outline_beat_mapping",
        "apply",
        &format!("part_id_{}", input.part_id),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "chapter_outline_beat_mapping",
            "mode": "apply",
            "project_id": input.project_id,
            "part_id": input.part_id,
            "beat_sheet": input.beat_sheet,
        }),
    )
    .await?;
    let result = state
        .outline
        .apply_beat_mapping(BeatMappingApplyInput {
            project_id: input.project_id,
            part_id: input.part_id,
            beat_sheet: &input.beat_sheet,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let outlines = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        advance_status(state, input.project_id, ProjectStatus::ChapterOutlinesReady).await,
    )
    .await?;
    Ok(outlines.iter().map(chapter_outline_summary).collect())
}

pub async fn preview_apply_beat_mapping_request(
    state: &AppState,
    user_id: i64,
    input: ApplyBeatMappingInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    let variables = HashMap::new();
    let request = state
        .outline
        .preview_beat_mapping_request(
            input.project_id,
            input.part_id,
            &input.beat_sheet,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    single_request_preview(
        state,
        "outline_beat_mapping",
        "节拍映射请求预览",
        "apply_beat_mapping",
        format!("分部 #{} 节拍映射", input.part_id),
        "outline",
        request,
    )
    .await
}

pub async fn generate_chapter_outlines_by_count(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterOutlinesByCountInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    report(
        progress,
        0,
        progress_total(input.count),
        "启动章节大纲追加生成",
    )?;
    let variables = HashMap::new();
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "chapter_outline_generation",
        "generate_by_count",
        &format!("chapter_{}", input.start_chapter),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "chapter_outline_generation",
            "mode": "generate_by_count",
            "project_id": input.project_id,
            "start_chapter": input.start_chapter,
            "count": input.count,
        }),
    )
    .await?;
    let result = state
        .outline
        .generate_chapter_outlines_by_count(ChapterOutlineCountGenerationInput {
            project_id: input.project_id,
            start_chapter: input.start_chapter,
            count: input.count,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let outlines = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        advance_status(state, input.project_id, ProjectStatus::ChapterOutlinesReady).await,
    )
    .await?;
    Ok(outlines.iter().map(chapter_outline_summary).collect())
}

pub async fn preview_generate_chapter_outlines_by_count_request(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterOutlinesByCountInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = HashMap::new();
    let steps = state
        .outline
        .preview_generate_chapter_outlines_by_count_requests(
            input.project_id,
            input.start_chapter,
            input.count,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    outline_preview(
        state,
        "chapter_outline_generation_by_count",
        "章节大纲追加生成请求预览",
        steps,
    )
    .await
}

pub async fn get_chapter_outlines_pacing(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Value, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let outlines = state.outline.list_chapter_outlines(project_id).await?;
    build_chapter_outlines_pacing(&outlines)
}

fn build_chapter_outlines_pacing(outlines: &[chapter_outline::Model]) -> Result<Value, AppError> {
    if outlines.is_empty() {
        return Ok(serde_json::json!({
            "pacing_score": 0.0,
            "analysis": [],
            "chapter_count": 0
        }));
    }

    let mut scores: Vec<f64> = Vec::new();
    let mut analysis = Vec::new();

    for outline in outlines {
        let pacing =
            required_outline_pacing_score(outline.pacing_score, outline.chapter_number)? as f64;
        scores.push(pacing);
        analysis.push(serde_json::json!({
            "chapter_number": outline.chapter_number,
            "title": outline.title,
            "pacing_score": pacing,
            "word_count_target": outline.word_count_target,
            "has_beats": outline.beats.is_some(),
            "has_key_scenes": outline.key_scenes.is_some(),
        }));
    }

    let avg_pacing: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    Ok(serde_json::json!({
        "pacing_score": (avg_pacing * 100.0).round() / 100.0,
        "chapter_count": outlines.len(),
        "analysis": analysis,
        "summary": if avg_pacing < 0.3 {
            "节奏偏慢"
        } else if avg_pacing > 0.7 {
            "节奏偏快"
        } else {
            "节奏适中"
        }
    }))
}

async fn outline_preview(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: Vec<OutlineRequestPreviewStep>,
) -> Result<ModelRequestPreview, AppError> {
    chat_status_preview_for_state(
        state,
        workflow,
        title,
        steps.into_iter().map(outline_preview_step),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}

async fn single_request_preview(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    key: impl Into<String>,
    title_step: impl Into<String>,
    prompt_name: impl Into<String>,
    request: ChatRequest,
) -> Result<ModelRequestPreview, AppError> {
    outline_preview(
        state,
        workflow,
        title,
        vec![OutlineRequestPreviewStep {
            key: key.into(),
            title: title_step.into(),
            prompt_name: prompt_name.into(),
            state: "ready".to_string(),
            note: None,
            request: Some(request),
        }],
    )
    .await
}

fn outline_preview_step(step: OutlineRequestPreviewStep) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        step.key,
        step.title,
        Some(step.prompt_name),
        step.state,
        step.note,
        step.request,
    )
}

async fn update_chapter_outline_record<C>(
    db: &C,
    input: UpdateChapterOutlineInput,
) -> Result<chapter_outline::Model, AppError>
where
    C: ConnectionTrait,
{
    let outline = chapter_outline::Entity::find_by_id(input.outline_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {}", input.outline_id)))?;
    ensure_chapter_outline_belongs_to_project(&outline, input.project_id)?;

    let mut active: chapter_outline::ActiveModel = outline.into();
    if let Some(title) = input.title {
        active.title = Set(Some(title));
    }
    if let Some(summary) = input.summary {
        active.summary = Set(Some(summary));
    }
    if let Some(beats) = input.beats {
        active.beats = Set(Some(beats));
    }
    if let Some(key_scenes) = input.key_scenes {
        active.key_scenes = Set(Some(key_scenes));
    }
    active.updated_at = Set(Utc::now().naive_utc());

    active.update(db).await.map_err(AppError::Database)
}

fn ensure_chapter_outline_belongs_to_project(
    outline: &chapter_outline::Model,
    project_id: i64,
) -> Result<(), AppError> {
    if outline.project_id == project_id {
        return Ok(());
    }
    Err(AppError::Forbidden(format!(
        "章节大纲 {} 不属于项目 {}",
        outline.id, project_id
    )))
}

async fn ensure_chapter_outline_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    outline_id: i64,
) -> Result<chapter_outline::Model, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let outline = chapter_outline::Entity::find_by_id(outline_id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {outline_id}")))?;
    ensure_chapter_outline_belongs_to_project(&outline, project_id)?;
    Ok(outline)
}

async fn ensure_part_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    part_id: i64,
) -> Result<part_outline::Model, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let part = state.outline.get_part(part_id).await?;
    if part.project_id != project_id {
        return Err(AppError::Forbidden(format!(
            "分部大纲 {part_id} 不属于项目 {project_id}"
        )));
    }
    Ok(part)
}

async fn collect_unique_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    part_ids: &[i64],
) -> Result<Vec<part_outline::Model>, AppError> {
    if part_ids.is_empty() {
        return Err(AppError::InvalidArgument("part_ids 不能为空".to_string()));
    }
    let mut seen = HashSet::new();
    let mut parts = Vec::with_capacity(part_ids.len());
    for part_id in part_ids {
        if !seen.insert(*part_id) {
            return Err(AppError::InvalidArgument(format!(
                "part_ids 包含重复分部: {part_id}"
            )));
        }
        parts.push(ensure_part_access(state, user_id, project_id, *part_id).await?);
    }
    order_parts_for_batch_generation(&mut parts);
    Ok(parts)
}

fn order_parts_for_batch_generation(parts: &mut [part_outline::Model]) {
    parts.sort_by_key(|part| (part.part_number, part.id));
}

async fn outline_project_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    total: u32,
) -> Result<OutlineTaskScope, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    Ok(OutlineTaskScope { project_id, total })
}

async fn advance_status(
    state: &AppState,
    project_id: i64,
    next_status: ProjectStatus,
) -> Result<(), AppError> {
    crate::workflow::advance_project_status_if_allowed(state, project_id, next_status).await
}

fn report(
    progress: Option<OutlineProgressReporter<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress(current, total.max(1), message)?;
    }
    Ok(())
}

fn progress_total(chapter_count: i32) -> u32 {
    chapter_count.max(1) as u32
}

fn required_outline_pacing_score(
    pacing_score: Option<f32>,
    chapter_number: i32,
) -> Result<f32, AppError> {
    let score = pacing_score.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "第 {chapter_number} 章缺少 pacing_score，无法分析节奏"
        ))
    })?;
    if score.is_finite() {
        Ok(score)
    } else {
        Err(AppError::InvalidArgument(format!(
            "第 {chapter_number} 章 pacing_score 不是有限数字，无法分析节奏"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    fn sample_outline(project_id: i64, pacing_score: Option<f32>) -> chapter_outline::Model {
        let now = NaiveDate::from_ymd_opt(2026, 6, 6)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        chapter_outline::Model {
            id: 42,
            project_id,
            part_id: None,
            chapter_number: 3,
            title: Some("旧标题".to_string()),
            summary: Some("旧概要".to_string()),
            beats: None,
            key_scenes: None,
            pacing_score,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: None,
            status: "draft".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_part(id: i64, part_number: i32) -> part_outline::Model {
        let now = NaiveDate::from_ymd_opt(2026, 6, 6)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        part_outline::Model {
            id,
            project_id: 1,
            part_number,
            title: format!("第 {part_number} 部"),
            start_chapter: (part_number - 1) * 10 + 1,
            end_chapter: part_number * 10,
            summary: Some("概要".to_string()),
            theme: None,
            key_events: None,
            character_focus: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_part_range(id: i64, part_number: i32, start: i32, end: i32) -> part_outline::Model {
        let mut part = sample_part(id, part_number);
        part.start_chapter = start;
        part.end_chapter = end;
        part
    }

    fn progress_outline(id: i64, chapter_number: i32, status: &str) -> chapter_outline::Model {
        let mut outline = sample_outline(1, None);
        outline.id = id;
        outline.chapter_number = chapter_number;
        outline.status = status.to_string();
        outline
    }

    #[test]
    fn update_chapter_outline_input_requires_project_id() {
        let err = serde_json::from_value::<UpdateChapterOutlineInput>(json!({
            "outline_id": 42,
            "title": "新标题"
        }))
        .expect_err("project_id must be required");

        assert!(err.to_string().contains("project_id"));
    }

    #[test]
    fn update_chapter_outline_input_rejects_unknown_fields() {
        let err = serde_json::from_value::<UpdateChapterOutlineInput>(json!({
            "project_id": 7,
            "outline_id": 42,
            "title": "新标题",
            "projectId": 7
        }))
        .expect_err("unknown fields must fail instead of being ignored");

        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn update_chapter_outline_input_accepts_only_valid_provided_fields() {
        let input = serde_json::from_value::<UpdateChapterOutlineInput>(json!({
            "project_id": 7,
            "outline_id": 42,
            "title": "  新标题  ",
            "summary": "新概要",
            "beats": ["推进"],
            "key_scenes": [{"title": "关键场景"}]
        }))
        .expect("valid provided fields should deserialize");

        assert_eq!(input.title.as_deref(), Some("新标题"));
        assert_eq!(input.summary.as_deref(), Some("新概要"));
        assert_eq!(input.beats, Some(json!(["推进"])));
        assert_eq!(input.key_scenes, Some(json!([{"title": "关键场景"}])));
    }

    #[test]
    fn update_chapter_outline_input_rejects_null_or_blank_text_fields() {
        for (field, value, expected) in [
            ("title", json!(null), "不能为 null"),
            ("summary", json!(" \n\t "), "不能为空"),
        ] {
            let mut payload = json!({
                "project_id": 7,
                "outline_id": 42
            });
            payload[field] = value;

            let err = serde_json::from_value::<UpdateChapterOutlineInput>(payload)
                .expect_err("invalid text field must fail");

            assert!(err.to_string().contains(field));
            assert!(err.to_string().contains(expected));
        }
    }

    #[test]
    fn update_chapter_outline_input_rejects_non_array_structured_fields() {
        for (field, value, expected) in [
            ("beats", json!("推进"), "必须是数组"),
            ("key_scenes", json!(null), "不能为 null"),
        ] {
            let mut payload = json!({
                "project_id": 7,
                "outline_id": 42
            });
            payload[field] = value;

            let err = serde_json::from_value::<UpdateChapterOutlineInput>(payload)
                .expect_err("invalid structured field must fail");

            assert!(err.to_string().contains(field));
            assert!(err.to_string().contains(expected));
        }
    }

    #[test]
    fn ensure_chapter_outline_belongs_to_project_rejects_mismatch() {
        let outline = sample_outline(1, None);

        let err = ensure_chapter_outline_belongs_to_project(&outline, 2)
            .expect_err("cross-project update must fail");

        assert!(err.to_string().contains("不属于项目 2"));
    }

    #[test]
    fn required_outline_pacing_score_rejects_missing_or_non_finite_score() {
        let missing = required_outline_pacing_score(None, 3)
            .expect_err("missing pacing score should be rejected");
        assert!(missing.to_string().contains("第 3 章缺少 pacing_score"));

        let invalid = required_outline_pacing_score(Some(f32::NAN), 4)
            .expect_err("non-finite pacing score should be rejected");
        assert!(
            invalid
                .to_string()
                .contains("第 4 章 pacing_score 不是有限数字")
        );
    }

    #[test]
    fn build_chapter_outlines_pacing_accepts_finite_scores() {
        let pacing = build_chapter_outlines_pacing(&[
            sample_outline(1, Some(0.5)),
            sample_outline(1, Some(0.7)),
        ])
        .expect("finite pacing scores should be accepted");

        assert_eq!(pacing["pacing_score"], json!(0.6));
        assert_eq!(pacing["summary"], json!("节奏适中"));
    }

    #[test]
    fn order_parts_for_batch_generation_uses_part_order_not_input_order() {
        let mut parts = vec![
            sample_part(30, 3),
            sample_part(10, 1),
            sample_part(20, 2),
            sample_part(19, 2),
        ];

        order_parts_for_batch_generation(&mut parts);

        let ids = parts.iter().map(|part| part.id).collect::<Vec<_>>();
        assert_eq!(ids, vec![10, 19, 20, 30]);
    }

    #[test]
    fn completed_part_count_requires_full_completed_chapter_range() {
        let parts = vec![
            sample_part_range(1, 1, 1, 2),
            sample_part_range(2, 2, 3, 4),
            sample_part_range(3, 3, 5, 5),
        ];
        let outlines = vec![
            progress_outline(11, 1, chapter_outline::STATUS_COMPLETED),
            progress_outline(12, 2, chapter_outline::STATUS_COMPLETED),
            progress_outline(13, 3, chapter_outline::STATUS_COMPLETED),
            progress_outline(14, 4, chapter_outline::STATUS_DRAFT),
            progress_outline(15, 5, chapter_outline::STATUS_COMPLETED),
        ];

        let completed =
            completed_part_count(&parts, &outlines).expect("valid progress data should count");

        assert_eq!(completed, 2);
    }

    #[test]
    fn completed_part_count_rejects_invalid_progress_state() {
        let duplicate = completed_part_count(
            &[sample_part_range(1, 1, 1, 2)],
            &[
                progress_outline(11, 1, chapter_outline::STATUS_COMPLETED),
                progress_outline(12, 1, chapter_outline::STATUS_COMPLETED),
            ],
        )
        .expect_err("duplicate chapter outlines must be explicit");
        assert!(duplicate.to_string().contains("存在重复章节大纲"));

        let overlapping = completed_part_count(
            &[sample_part_range(1, 1, 1, 2), sample_part_range(2, 2, 2, 3)],
            &[],
        )
        .expect_err("overlapping part ranges must be explicit");
        assert!(overlapping.to_string().contains("被多个分部范围覆盖"));
    }
}
