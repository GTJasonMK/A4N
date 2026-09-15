use std::time::Instant;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use a4n_provider_runtime_contract::TaskLease;
use a4n_queue::priority::TaskPriority;

use super::super::{
    budget::requested_max_tokens,
    config::require_sidecar_business_trace,
    contracts::{DynamicLlmProvider, ResolvedLlmModel},
    trace::prepare_dynamic_llm_trace_context,
};

pub(in crate::dynamic_llm) struct PreparedLlmCall {
    pub model: ResolvedLlmModel,
    pub request: ChatRequest,
    pub trace: Option<LlmTraceContext>,
    pub task: Box<dyn TaskLease>,
    pub started: Instant,
}

impl DynamicLlmProvider {
    pub(in crate::dynamic_llm) async fn prepare_call(
        &self,
        model: ResolvedLlmModel,
        mut request: ChatRequest,
    ) -> Result<PreparedLlmCall, AppError> {
        request.max_tokens = Some(requested_max_tokens(
            request.max_tokens,
            model.capabilities.max_output_tokens,
            model.model_name(),
        )?);
        let trace =
            prepare_dynamic_llm_trace_context(&self.facade, &model.planner(), &mut request).await?;
        request.configure_prompt_cache(
            &model.model.provider,
            model.model_name(),
            model.model.extra_body_json.as_ref(),
            trace
                .as_ref()
                .and_then(|trace| trace.root_project_id.or(trace.project_id)),
        )?;
        if self.sidecar_client.is_some() {
            require_sidecar_business_trace(trace.as_ref(), "LLM sidecar stream 请求")?;
        }
        request.stream_include_usage = true;
        tracing::debug!(target: "a4n::llm", model = model.model_name(), config_id = model.model.id, max_tokens = ?request.max_tokens, "LLM stream 请求");
        let started = Instant::now();
        let task = self
            .facade
            .queue_start_llm_task(
                model.concurrency_limit(),
                TaskPriority::Medium,
                trace.as_ref(),
            )
            .await?;
        Ok(PreparedLlmCall {
            model,
            request,
            trace,
            task,
            started,
        })
    }
}
