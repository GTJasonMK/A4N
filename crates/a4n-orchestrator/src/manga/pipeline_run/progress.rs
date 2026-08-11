use a4n_core::error::AppError;
use a4n_db::workflow::WorkflowProgressEventInput;
use a4n_llm::trace::llm_usage_summary_text;

use super::super::pipeline_prompt::MangaPromptGenerationTrace;
use super::super::{MANGA_PIPELINE_TOTAL_STEPS, OrchestratorContext};

impl OrchestratorContext {
    pub(in crate::manga) async fn record_manga_pipeline_progress(
        &self,
        workflow_run_id: i64,
        task_id: &str,
        stage: &str,
        current: i32,
        message: &str,
    ) -> Result<(), AppError> {
        self.mark_workflow_current_step(workflow_run_id, stage)
            .await?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: Some(task_id),
            current: u32::try_from(current).map_err(|_| {
                AppError::InvalidArgument(format!("漫画流水线进度不能为负数: {current}"))
            })?,
            total: u32::try_from(MANGA_PIPELINE_TOTAL_STEPS)
                .map_err(|_| AppError::InvalidArgument("漫画流水线总步数不能为负数".to_string()))?,
            message,
            meta: None,
        })
        .await?;
        Ok(())
    }

    pub(in crate::manga::pipeline_run) async fn report_pipeline_llm_usage(
        &self,
        workflow_run_id: i64,
        stage: &str,
        current: i32,
        task_id: &str,
        trace: &MangaPromptGenerationTrace,
    ) -> Result<(), AppError> {
        let message = format!(
            "模型请求完成：{}；{}；正在校验结构",
            trace.prompt_name,
            llm_usage_summary_text(trace.usage.as_ref())
        );
        self.record_manga_pipeline_progress(workflow_run_id, task_id, stage, current, &message)
            .await
    }
}
