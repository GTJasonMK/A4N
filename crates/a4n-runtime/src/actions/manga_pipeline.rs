use crate::AppState;
use crate::actions::manga_source::{
    collect_manga_reference_context, ensure_manga_source_can_write,
    resolve_chapter_manga_source_by_number, verify_manga_project_editor,
};
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_services::manga::{
    ChapterMangaSourceContext, MangaGeneratePanelsInput, MangaPipelineRequestPreviewInput,
    MangaPipelineStartStage, MangaReferenceContext,
};
use a4n_services::source::{ChapterSourceContext, source_metadata};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use std::collections::HashMap;

pub fn parse_manga_pipeline_start_stage(
    value: Option<String>,
) -> Result<Option<MangaPipelineStartStage>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    MangaPipelineStartStage::parse(value)
        .map(Some)
        .ok_or_else(|| AppError::InvalidArgument(format!("未知漫画流水线起始步骤: {value}")))
}

pub async fn preview_manga_pipeline_requests(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
) -> Result<ModelRequestPreview, AppError> {
    verify_manga_project_editor(state, user_id, project_id).await?;
    let source =
        resolve_chapter_manga_source_by_number(state, project_id, chapter_number, true).await?;
    ensure_manga_source_can_write(&source, "预览漫画流水线提示词")?;
    let reference_context = collect_manga_reference_context(state, project_id).await?;
    let workflow_fingerprint = manga_pipeline_fingerprint(&source.content, &reference_context);
    let workflow_run_id = state
        .workflow_run
        .latest_run_by_business_key(&manga_pipeline_business_key(project_id, source.chapter_id))
        .await?
        .filter(|run| run.input_fingerprint_json.as_ref() == Some(&workflow_fingerprint))
        .map(|run| run.id);
    let variables = HashMap::new();
    let steps = state
        .manga
        .preview_generate_panels_requests(MangaPipelineRequestPreviewInput {
            chapter_id: source.chapter_id,
            project_id,
            source: ChapterMangaSourceContext {
                content: &source.content,
            },
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            reference_context,
            workflow_run_id,
            resume_from_checkpoint,
            start_from_stage,
        })
        .await?;
    chat_status_preview_for_state(
        state,
        "manga_pipeline",
        format!("第 {chapter_number} 章漫画流水线请求预览"),
        steps.into_iter().map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        }),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}

pub async fn run_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    task_id: &str,
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    verify_manga_project_editor(state, user_id, project_id).await?;
    let source =
        resolve_chapter_manga_source_by_number(state, project_id, chapter_number, true).await?;
    ensure_manga_source_can_write(&source, "启动漫画流水线")?;
    let reference_context = collect_manga_reference_context(state, project_id).await?;
    let workflow_fingerprint = manga_pipeline_fingerprint(&source.content, &reference_context);
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: manga_pipeline_business_key(project_id, source.chapter_id),
            workflow_kind: "manga_pipeline".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(source.chapter_id),
            chapter_id: Some(source.chapter_id),
            mode: start_from_stage.map(manga_start_stage_key),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: start_from_stage.map(manga_start_stage_key),
            active_task_id: Some(task_id.to_string()),
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let variables = HashMap::new();
    let result = state
        .manga
        .generate_panels(MangaGeneratePanelsInput {
            chapter_id: source.chapter_id,
            project_id,
            source: ChapterMangaSourceContext {
                content: &source.content,
            },
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            reference_context,
            task_id: Some(task_id),
            workflow_run_id: Some(workflow_run.id),
            resume_from_checkpoint,
            start_from_stage,
        })
        .await;
    match &result {
        Ok(_) => {
            state
                .workflow_run
                .mark_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await?;
        }
        Err(err) => {
            let status = if matches!(err, AppError::Cancelled) {
                WORKFLOW_STATUS_CANCELLED
            } else {
                WORKFLOW_STATUS_FAILED
            };
            state
                .workflow_run
                .mark_status(workflow_run.id, status, true, Some(&err.to_string()))
                .await
                .map_err(|status_err| {
                    AppError::InvalidArgument(format!(
                        "{err}; 同步漫画工作流状态失败: {status_err}"
                    ))
                })?;
        }
    }
    result
}

pub async fn preflight_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), AppError> {
    verify_manga_project_editor(state, user_id, project_id).await?;
    let source =
        resolve_chapter_manga_source_by_number(state, project_id, chapter_number, true).await?;
    ensure_manga_source_can_write(&source, "启动漫画流水线")
}

fn manga_pipeline_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("manga_pipeline:{project_id}:{chapter_id}")
}

fn manga_pipeline_fingerprint(
    content: &str,
    reference_context: &MangaReferenceContext,
) -> serde_json::Value {
    serde_json::json!({
        "source": source_metadata(ChapterSourceContext { content }),
        "reference_assets": {
            "character_portrait_count": reference_context.character_portraits.len(),
            "character_portrait_ids": reference_context
                .character_portraits
                .iter()
                .map(|portrait| portrait.portrait_id)
                .collect::<Vec<_>>(),
        }
    })
}

fn manga_start_stage_key(stage: MangaPipelineStartStage) -> String {
    match stage {
        MangaPipelineStartStage::EventBeats => "event_beats",
        MangaPipelineStartStage::CharacterBinding => "character_binding",
        MangaPipelineStartStage::SpeechSelection => "speech_selection",
        MangaPipelineStartStage::VisualAnchorExtraction => "visual_anchor_extraction",
        MangaPipelineStartStage::ItemExtraction => "item_extraction",
        MangaPipelineStartStage::ComicSynthesis => "comic_synthesis",
        MangaPipelineStartStage::PagePlanning => "page_planning",
        MangaPipelineStartStage::DrawingPrompt => "drawing_prompt",
    }
    .to_string()
}
