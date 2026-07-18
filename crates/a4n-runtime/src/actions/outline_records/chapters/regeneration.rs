use super::super::{
    ChapterOutlineSummary, OutlineProgressReporter, RegenerateChapterOutlineInput, advance_status,
    chapter_outline_summary, ensure_chapter_outline_access, finish_outline_postprocess,
    parse_template_variables, report, single_request_preview, start_outline_workflow_run,
    unwrap_outline_workflow_result,
};
use crate::AppState;
use crate::actions::request_preview::ModelRequestPreview;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;
use a4n_orchestrator::outline::ChapterOutlineRegenerationInput;
use serde_json::json;

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
        .orchestrator
        .regenerate_chapter_outline(ChapterOutlineRegenerationInput {
            project_id: input.project_id,
            outline_id: input.outline_id,
            prompt_renderer: state,
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
        .orchestrator
        .preview_regenerate_chapter_outline_request(
            input.project_id,
            input.outline_id,
            state,
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
