use super::super::{
    ApplyBeatMappingInput, ChapterOutlineSummary, OutlineProgressReporter, advance_status,
    chapter_outline_summary, ensure_part_access, finish_outline_postprocess, report,
    single_request_preview, start_outline_workflow_run, unwrap_outline_workflow_result,
};
use crate::AppState;
use crate::actions::request_preview::ModelRequestPreview;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;
use a4n_orchestrator::outline::BeatMappingApplyInput;
use serde_json::json;
use std::collections::HashMap;

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
        .orchestrator
        .apply_beat_mapping(BeatMappingApplyInput {
            project_id: input.project_id,
            part_id: input.part_id,
            beat_sheet: &input.beat_sheet,
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

pub async fn preview_apply_beat_mapping_request(
    state: &AppState,
    user_id: i64,
    input: ApplyBeatMappingInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    let variables = HashMap::new();
    let request = state
        .orchestrator
        .preview_beat_mapping_request(
            input.project_id,
            input.part_id,
            &input.beat_sheet,
            state,
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
