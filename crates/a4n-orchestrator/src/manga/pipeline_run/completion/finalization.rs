use a4n_core::error::AppError;

use super::super::super::PIPELINE_TOTAL_STEPS;
use super::super::super::pipeline_status::MangaPipelineStatusInput;
use super::super::contracts::MangaRunContext;

pub(super) async fn finalize_completed_pipeline(
    context: &MangaRunContext<'_>,
    result_count: usize,
) -> Result<(), AppError> {
    let service = context.service;
    service
        .ensure_pipeline_not_cancelled(context.project_id, context.chapter_id)
        .await?;
    service
        .set_pipeline_status(MangaPipelineStatusInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            pipeline_status: "completed",
            stage: "completed",
            current: PIPELINE_TOTAL_STEPS,
            total: PIPELINE_TOTAL_STEPS,
            message: &format!("漫画流水线完成，生成 {result_count} 页整页提示词"),
            task_id: context.task_id,
            allow_cancel_override: false,
        })
        .await?;
    service
        .clear_pipeline_checkpoint(context.project_id, context.chapter_id)
        .await
}
