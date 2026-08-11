use a4n_core::error::AppError;

use super::super::super::MANGA_PIPELINE_TOTAL_STEPS;
use super::super::contracts::MangaRunContext;

pub(super) async fn finalize_completed_pipeline(
    context: &MangaRunContext<'_>,
    result_count: usize,
) -> Result<(), AppError> {
    let service = context.service;
    let message = format!("漫画流水线完成，生成 {result_count} 页整页提示词");
    service
        .record_manga_pipeline_progress(
            context.workflow_run_id,
            context.task_id,
            "completed",
            MANGA_PIPELINE_TOTAL_STEPS,
            &message,
        )
        .await
}
