//! 大纲生成 workflow 入口。

use a4n_error::AppError;
use serde_json::Value;

use super::OrchestratorContext;

impl OrchestratorContext {
    /// 大纲 application 内部创建/恢复 workflow_run（RUNNING）。
    pub(crate) async fn begin_outline_workflow_validated(
        &self,
        project_id: i64,
        workflow_kind: &str,
        mode: &str,
        current_step_key: &str,
        task_id: Option<String>,
        input_fingerprint_json: Value,
    ) -> Result<a4n_domain::workflow::WorkflowRunSummary, AppError> {
        let input = crate::outline::outline_workflow_run_input(
            project_id,
            workflow_kind,
            mode,
            current_step_key,
            task_id,
            input_fingerprint_json,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_or_resume_workflow_run_validated(input).await
    }
}
