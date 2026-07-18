use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::PIPELINE_TOTAL_STEPS;
use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_json::parse_manga_page_plan;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;
use super::super::super::pipeline_status::MangaPipelineStatusInput;
use super::super::contracts::{
    MangaExtractionRun, MangaPlanningRun, MangaRunContext, MangaRunningPipelineCheckpointInput,
};

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
    service
        .save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            stage: "page_planning",
            current: 6,
            message: "已完成漫画分页规划",
            task_id: context.task_id,
            source_metadata: &context.source_metadata,
            reference_assets_metadata: &context.reference_assets_metadata,
            prompt_fingerprint: &context.prompt_fingerprint,
            checkpoint,
        })
        .await?;
    service
        .ensure_pipeline_not_cancelled(context.project_id, context.chapter_id)
        .await?;
    service
        .set_pipeline_status(MangaPipelineStatusInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            pipeline_status: "running",
            stage: "drawing_prompt",
            current: 7,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在生成整页漫画提示词",
            task_id: context.task_id,
            allow_cancel_override: false,
        })
        .await?;

    Ok(MangaPlanningRun {
        extraction,
        plan_json,
    })
}
