use a4n_error::AppError;
use serde_json::Value;

use crate::OrchestratorContext;
use a4n_prompt::rendering::PromptRenderer;

mod execution;
mod preparation;
mod restoration;

use execution::execute_workflow_llm_once;
use preparation::prepare_workflow_llm_once;
use restoration::restore_workflow_llm_once;

pub(crate) const WORKFLOW_LLM_TEMPERATURE: f32 = 0.25;

pub(crate) fn workflow_json_request(
    system: String,
    user: String,
) -> a4n_llm_provider_contract::provider::ChatRequest {
    use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest};
    ChatRequest::new(
        vec![ChatMessage::system(system), ChatMessage::user(user)],
        Some(WORKFLOW_LLM_TEMPERATURE),
        None,
    )
    .with_json_response_format()
}

pub(crate) struct WorkflowLlmOnceInput<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub project_id: i64,
    pub root_project_id: i64,
    pub workflow_run_id: i64,
    pub workflow: &'a str,
    pub step_key: String,
    pub stage_key: &'a str,
    pub call_order: i32,
    pub prompt_name: &'a str,
    pub user_prompt: String,
}

struct PreparedWorkflowLlmOnce {
    system_prompt: String,
    fingerprint_json: Value,
    input_fingerprint_sha256: String,
}

impl OrchestratorContext {
    pub(crate) async fn run_workflow_llm_once<T, F>(
        &self,
        input: WorkflowLlmOnceInput<'_>,
        validate: F,
    ) -> Result<(Value, T), AppError>
    where
        T: serde::Serialize,
        F: Fn(Value) -> Result<T, AppError>,
    {
        self.run_workflow_llm_once_with_stream_reporter(input, None, validate)
            .await
    }

    pub(crate) async fn run_workflow_llm_once_with_stream_reporter<T, F>(
        &self,
        input: WorkflowLlmOnceInput<'_>,
        stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
        validate: F,
    ) -> Result<(Value, T), AppError>
    where
        T: serde::Serialize,
        F: Fn(Value) -> Result<T, AppError>,
    {
        if self
            .workflow_run_status_record_by_id(input.workflow_run_id)
            .await?
            .status
            == super::WORKFLOW_STATUS_CANCELLED
        {
            return Err(AppError::Cancelled);
        }
        let prepared = prepare_workflow_llm_once(&input).await?;
        self.mark_workflow_current_step(input.workflow_run_id, input.stage_key)
            .await?;
        if let Some(restored) =
            restore_workflow_llm_once(self, &input, &prepared, &validate).await?
        {
            return Ok(restored);
        }
        execute_workflow_llm_once(self, &input, prepared, stream_reporter, validate).await
    }
}
