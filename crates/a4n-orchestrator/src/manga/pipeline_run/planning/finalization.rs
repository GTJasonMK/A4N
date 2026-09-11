use a4n_error::AppError;

use super::super::super::MangaPipelineStage;
use super::super::super::pipeline_json::parse_manga_page_plan;
use super::super::super::pipeline_prompt::{
    MangaPromptGenerationTrace, parse_and_validate_manga_llm_output,
};
use super::super::contracts::{MangaExtractionRun, MangaPlanningRun, MangaRunContext};

pub(super) async fn finalize_planning(
    context: &MangaRunContext<'_>,
    extraction: MangaExtractionRun,
    plan_trace: MangaPromptGenerationTrace,
) -> Result<MangaPlanningRun, AppError> {
    let service = context.service;
    service
        .report_pipeline_llm_usage(
            context.workflow_run_id,
            "page_planning",
            6,
            context.task_id,
            &plan_trace,
        )
        .await?;
    let (plan, plan_json) = parse_and_validate_manga_llm_output(&plan_trace, |value| {
        let plan = parse_manga_page_plan(value, &context.shooting_script, "manga_page_planning")?;
        let plan_json = serde_json::to_value(&plan).map_err(AppError::Serialization)?;
        Ok((plan, plan_json))
    })?;
    service
        .save_or_validate_manga_llm_checkpoint(&plan_trace, &plan_json)
        .await?;
    context
        .advance_stage(MangaPipelineStage::DrawingPrompt)
        .await?;

    Ok(MangaPlanningRun { extraction, plan })
}
