use super::super::{
    ChapterOutlineSummary, GeneratePartChapterOutlinesInput, OutlineProgressReporter,
    advance_status, chapter_outline_summary, ensure_part_access, finish_outline_postprocess,
    outline_preview, part_chapter_count, progress_total, report, start_outline_workflow_run,
    unwrap_outline_workflow_result,
};
use crate::AppState;
use crate::actions::request_preview::ModelRequestPreview;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;
use a4n_orchestrator::outline::ChapterOutlineGenerationInput;
use serde_json::json;
use std::collections::HashMap;

pub async fn generate_part_chapter_outlines(
    state: &AppState,
    user_id: i64,
    input: GeneratePartChapterOutlinesInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    let part = ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    let count = part_chapter_count(part.start_chapter, part.end_chapter)?;
    report(
        progress,
        0,
        progress_total(count)?,
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
        .orchestrator
        .generate_chapter_outlines(ChapterOutlineGenerationInput {
            project_id: input.project_id,
            part_id: part.id,
            start_chapter: part.start_chapter,
            count,
            prompt_renderer: state,
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
        .orchestrator
        .preview_generate_part_chapter_outline_requests(
            input.project_id,
            input.part_id,
            state,
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
