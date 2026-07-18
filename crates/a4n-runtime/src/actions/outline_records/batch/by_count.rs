use super::super::{
    ChapterOutlineSummary, GenerateChapterOutlinesByCountInput, OutlineProgressReporter,
    advance_status, chapter_outline_summary, finish_outline_postprocess, outline_preview,
    progress_total, report, start_outline_workflow_run, unwrap_outline_workflow_result,
};
use crate::AppState;
use crate::actions::access as project_access;
use crate::actions::request_preview::ModelRequestPreview;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;
use a4n_orchestrator::outline::ChapterOutlineCountGenerationInput;
use serde_json::json;
use std::collections::HashMap;

pub async fn generate_chapter_outlines_by_count(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterOutlinesByCountInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    report(
        progress,
        0,
        progress_total(input.count)?,
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
        .orchestrator
        .generate_chapter_outlines_by_count(ChapterOutlineCountGenerationInput {
            project_id: input.project_id,
            start_chapter: input.start_chapter,
            count: input.count,
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

pub async fn preview_generate_chapter_outlines_by_count_request(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterOutlinesByCountInput,
) -> Result<ModelRequestPreview, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = HashMap::new();
    let steps = state
        .orchestrator
        .preview_generate_chapter_outlines_by_count_requests(
            input.project_id,
            input.start_chapter,
            input.count,
            state,
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
