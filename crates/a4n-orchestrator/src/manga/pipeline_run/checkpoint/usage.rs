use a4n_core::error::AppError;
use a4n_llm::trace::llm_usage_summary_text;

use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;
use super::super::super::pipeline_status::MangaPipelineStatusInput;
use super::super::super::{OrchestratorContext, PIPELINE_TOTAL_STEPS};

impl OrchestratorContext {
    pub(in crate::manga::pipeline_run) async fn report_pipeline_llm_usage(
        &self,
        chapter_id: i64,
        project_id: i64,
        stage: &str,
        current: i32,
        task_id: Option<&str>,
        trace: &MangaPromptGenerationTrace,
    ) -> Result<(), AppError> {
        let message = format!(
            "模型请求完成：{}；{}；正在校验结构，必要时自动修复",
            trace.prompt_name,
            llm_usage_summary_text(trace.usage.as_ref())
        );
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage,
            current,
            total: PIPELINE_TOTAL_STEPS,
            message: &message,
            task_id,
            allow_cancel_override: true,
        })
        .await
    }
}
