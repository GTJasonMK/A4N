use a4n_domain::manga::MangaPagePlanV3;
use a4n_error::AppError;

use super::super::super::super::MangaPipelineStage;
use super::super::super::prompt::MangaPromptGenerationTrace;
use super::super::contracts::{MangaExtractionRun, MangaPlanningRun, MangaRunContext};

pub(super) async fn finalize_planning(
    context: &MangaRunContext<'_>,
    extraction: MangaExtractionRun,
    plan_trace: MangaPromptGenerationTrace,
    plan: MangaPagePlanV3,
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
    context
        .advance_stage(MangaPipelineStage::DrawingPrompt)
        .await?;

    Ok(MangaPlanningRun { extraction, plan })
}
