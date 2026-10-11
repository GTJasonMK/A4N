use crate::provider_runtime::DynamicProviderFacade;
use a4n_domain::llm::RecordLlmCallTraceInput;
use a4n_llm_provider_contract::provider::{LlmTraceContext, UsageInfo};

pub(in crate::dynamic_llm) struct DynamicLlmTraceOutcome<'a> {
    pub(in crate::dynamic_llm) model: &'a str,
    pub(in crate::dynamic_llm) usage: Option<UsageInfo>,
    pub(in crate::dynamic_llm) duration_ms: Option<i64>,
    pub(in crate::dynamic_llm) status: &'a str,
    pub(in crate::dynamic_llm) error_kind: Option<&'a str>,
    pub(in crate::dynamic_llm) error_message: Option<&'a str>,
    pub(in crate::dynamic_llm) finish_reason: Option<&'a str>,
}

pub(in crate::dynamic_llm) async fn record_dynamic_llm_trace(
    facade: &DynamicProviderFacade,
    trace_context: Option<&LlmTraceContext>,
    outcome: DynamicLlmTraceOutcome<'_>,
) {
    let Some(trace_context) = trace_context else {
        return;
    };
    let DynamicLlmTraceOutcome {
        model,
        usage,
        duration_ms,
        status,
        error_kind,
        error_message,
        finish_reason,
    } = outcome;
    let result = facade
        .record_llm_call_trace(RecordLlmCallTraceInput {
            generation_run_id: trace_context.generation_run_id,
            workflow_run_id: trace_context.workflow_run_id,
            call_key: trace_context.call_key.clone(),
            input_fingerprint_sha256: trace_context.input_fingerprint_sha256.clone(),
            project_id: trace_context.project_id,
            root_project_id: trace_context.root_project_id,
            workflow: trace_context.workflow.clone(),
            step_key: trace_context.step_key.clone(),
            provider: None,
            model: Some(model.to_string()),
            context_plan_json: trace_context.context_plan_json.clone(),
            context_pack_refs_json: trace_context.context_pack_refs_json.clone(),
            usage,
            duration_ms,
            streamed: true,
            status: status.to_string(),
            error_kind: error_kind.map(ToString::to_string),
            error_message: error_message.map(ToString::to_string),
            finish_reason: finish_reason.map(ToString::to_string),
        })
        .await;
    if let Err(err) = result {
        tracing::warn!(
            target: "a4n::llm::trace",
            workflow = %trace_context.workflow,
            step_key = %trace_context.step_key,
            error = %err,
            "记录 LLM 调用 trace 失败"
        );
    }
}
