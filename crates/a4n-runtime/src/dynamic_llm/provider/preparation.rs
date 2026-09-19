use std::time::Instant;

use a4n_domain::cost::{ModelCapability, ModelUsageRecordStart};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use a4n_provider_runtime_contract::{ModelUsageRecordHandle, TaskLease};
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
    pub cost_record: ModelUsageRecordHandle,
    pub started: Instant,
}

impl DynamicLlmProvider {
    pub(in crate::dynamic_llm) async fn prepare_request(
        &self,
        model: ResolvedLlmModel,
        mut request: ChatRequest,
    ) -> Result<(ResolvedLlmModel, ChatRequest, Option<LlmTraceContext>), AppError> {
        request.max_tokens = Some(requested_max_tokens(
            request.max_tokens,
            model.capabilities.max_output_tokens,
            model.model_name(),
        )?);
        let trace = if request.context_manifest_json.is_some()
            && request.trace_context.as_ref().is_some_and(|trace| {
                trace.context_plan_json.is_some() && trace.context_pack_refs_json.is_some()
            }) {
            request.trace_context.clone()
        } else {
            prepare_dynamic_llm_trace_context(&self.facade, &model.planner(), &mut request).await?
        };
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
        Ok((model, request, trace))
    }

    pub(in crate::dynamic_llm) async fn prepare_call(
        &self,
        model: ResolvedLlmModel,
        request: ChatRequest,
    ) -> Result<PreparedLlmCall, AppError> {
        let (model, request, trace) = self.prepare_request(model, request).await?;
        tracing::info!(
            target: "a4n::llm",
            model = model.model_name(),
            config_id = model.model.id,
            max_context_tokens = model.capabilities.max_context_tokens,
            configured_max_output_tokens = model.capabilities.max_output_tokens,
            request_max_tokens = ?request.max_tokens,
            "LLM stream 使用运行时配置"
        );
        let started = Instant::now();
        let trace_for_cost = trace.as_ref();
        let cost_record = self
            .facade
            .start_model_usage(ModelUsageRecordStart {
                request_id: uuid::Uuid::new_v4().to_string(),
                user_id: crate::runtime_config::current_user_id(),
                project_id: trace_for_cost.and_then(|trace| trace.project_id),
                root_project_id: trace_for_cost.and_then(|trace| trace.root_project_id),
                workflow_run_id: trace_for_cost.and_then(|trace| trace.workflow_run_id),
                generation_run_id: trace_for_cost.and_then(|trace| trace.generation_run_id),
                workflow: trace_for_cost
                    .map(|trace| trace.workflow.clone())
                    .unwrap_or_else(|| "llm".to_string()),
                step_key: trace_for_cost
                    .map(|trace| trace.step_key.clone())
                    .unwrap_or_else(|| "chat".to_string()),
                call_key: trace_for_cost.and_then(|trace| trace.call_key.clone()),
                source: Some("llm_call".to_string()),
                capability: ModelCapability::Llm,
                provider_config_id: Some(model.model.id),
                provider: Some(model.model.provider.clone()),
                model: Some(model.model.model.clone()),
                pricing: model.model.pricing.clone(),
            })
            .await?;
        let task = match self
            .facade
            .queue_start_llm_task(
                model.concurrency_limit(),
                TaskPriority::Medium,
                trace.as_ref(),
            )
            .await
        {
            Ok(task) => task,
            Err(error) => {
                let _ = self
                    .facade
                    .finish_model_usage(
                        cost_record,
                        a4n_domain::cost::ModelUsageRecordCompletion {
                            status: "failed".to_string(),
                            meter: Default::default(),
                            error_kind: Some("queue".to_string()),
                            error_message: Some(error.to_string()),
                        },
                    )
                    .await;
                return Err(error);
            }
        };
        Ok(PreparedLlmCall {
            model,
            request,
            trace,
            task,
            cost_record,
            started,
        })
    }
}
