use super::super::super::{MANGA_PIPELINE_TOTAL_STEPS, OrchestratorContext};
use a4n_db::workflow::WorkflowProgressEventInput;
use a4n_error::AppError;

impl OrchestratorContext {
    pub(in crate::manga) async fn record_manga_pipeline_progress(
        &self,
        workflow_run_id: i64,
        task_id: &str,
        stage: &str,
        current: i32,
        message: &str,
    ) -> Result<(), AppError> {
        self.record_manga_pipeline_progress_with_total(
            workflow_run_id,
            task_id,
            stage,
            current,
            MANGA_PIPELINE_TOTAL_STEPS,
            message,
        )
        .await
    }

    pub(in crate::manga) async fn record_manga_pipeline_progress_with_total(
        &self,
        workflow_run_id: i64,
        task_id: &str,
        stage: &str,
        current: i32,
        total: i32,
        message: &str,
    ) -> Result<(), AppError> {
        if current < 0 || total <= 0 || current > total {
            return Err(AppError::InvalidArgument(format!(
                "漫画流水线进度无效: current={current}, total={total}"
            )));
        }
        self.mark_workflow_current_step(workflow_run_id, stage)
            .await?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: Some(task_id),
            current: u32::try_from(current).map_err(|_| {
                AppError::InvalidArgument(format!("漫画流水线进度不能为负数: {current}"))
            })?,
            total: u32::try_from(total)
                .map_err(|_| AppError::InvalidArgument("漫画流水线总步数不能为负数".to_string()))?,
            message,
            meta: None,
        })
        .await?;
        Ok(())
    }
}
