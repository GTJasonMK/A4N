use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::MangaPipelineStartStage;
use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_json::parse_manga_page_plan;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;
use super::super::contracts::{MangaExtractionRun, MangaPlanningRun, MangaRunContext};

pub(super) async fn finalize_planning(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    extraction: MangaExtractionRun,
    plan_trace: MangaPromptGenerationTrace,
) -> Result<MangaPlanningRun, AppError> {
    let service = context.service;
    service
        .report_pipeline_llm_usage(
            context.chapter_id,
            context.project_id,
            "page_planning",
            6,
            context.task_id,
            &plan_trace,
        )
        .await?;
    let plan_out = plan_trace.response_text.clone();
    checkpoint_insert_trace(checkpoint, "plan_trace", &plan_trace)?;
    let plan_json = service
        .repair_and_validate_manga_llm_output(&plan_trace, |value| {
            let typed_plan =
                parse_manga_page_plan(value, &context.shooting_script, "manga_page_planning")?;
            serde_json::to_value(typed_plan).map_err(AppError::Serialization)
        })
        .await?;
    service
        .save_or_validate_manga_llm_checkpoint(&plan_trace, &plan_json)
        .await?;
    checkpoint_insert(checkpoint, "plan_out", json!(plan_out.clone()));
    checkpoint_insert(checkpoint, "plan_json", plan_json.clone());
    context
        .checkpoint_and_advance_stage(
            checkpoint,
            MangaPipelineStartStage::PagePlanning,
            "已完成漫画分页规划",
            MangaPipelineStartStage::DrawingPrompt,
        )
        .await?;

    Ok(MangaPlanningRun {
        extraction,
        plan_json,
    })
}
