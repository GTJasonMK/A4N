use super::super::{
    BatchGeneratePartOutlinesInput, ChapterOutlineSummary, OutlineProgressReporter, advance_status,
    batch_parts_progress_total, chapter_outline_summary, collect_unique_parts,
    finish_outline_postprocess, outline_preview, part_chapter_count, progress_total, report,
    start_outline_workflow_run, unwrap_outline_workflow_result,
};
use crate::AppState;
use crate::actions::request_preview::ModelRequestPreview;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;
use a4n_orchestrator::outline::ChapterOutlineGenerationInput;
use serde_json::json;
use std::collections::HashMap;

pub async fn batch_generate_part_outlines(
    state: &AppState,
    user_id: i64,
    input: BatchGeneratePartOutlinesInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    let parts = collect_unique_parts(state, user_id, input.project_id, &input.part_ids).await?;
    let total = batch_parts_progress_total(&parts)?;
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
            let count = part_chapter_count(part.start_chapter, part.end_chapter)?;
            let part_total = progress_total(count)?;
            let offset_progress = |current: u32, child_total: u32, message: &str| {
                if child_total != part_total || current > child_total {
                    return Err(AppError::InvalidArgument(format!(
                        "分部章节大纲进度不一致：current={current}, total={child_total}, expected_total={part_total}"
                    )));
                }
                let batch_current = completed.checked_add(current).ok_or_else(|| {
                    AppError::InvalidArgument("批量章节大纲进度超出 u32 范围".to_string())
                })?;
                report(progress, batch_current, total, message)
            };
            let outlines = state
                .orchestrator
                .generate_chapter_outlines(ChapterOutlineGenerationInput {
                    project_id: input.project_id,
                    part_id: part.id,
                    start_chapter: part.start_chapter,
                    count,
                    prompt_renderer: state,
                    variables: &variables,
                    progress: Some(&offset_progress),
                    workflow_run_id: Some(workflow_run_id),
                })
                .await?;
            completed = completed.checked_add(part_total).ok_or_else(|| {
                AppError::InvalidArgument("批量章节大纲进度超出 u32 范围".to_string())
            })?;
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
            .orchestrator
            .preview_generate_part_chapter_outline_requests(
                input.project_id,
                part.id,
                state,
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
