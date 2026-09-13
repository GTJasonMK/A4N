use crate::provider_runtime::DynamicProviderFacade;
use a4n_domain::llm::RecordLlmCallTraceInput;
use a4n_llm::provider::{LlmTraceContext, UsageInfo};

pub(in crate::dynamic_llm) async fn record_dynamic_llm_trace(
    facade: &DynamicProviderFacade,
    trace_context: Option<&LlmTraceContext>,
    model: Option<&str>,
    usage: Option<UsageInfo>,
    duration_ms: Option<i64>,
    streamed: bool,
    status: &str,
    error_kind: Option<&str>,
    error_message: Option<&str>,
    finish_reason: Option<&str>,
) {
    let Some(trace_context) = trace_context else {
        return;
    };
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
            model: model.map(ToString::to_string),
            context_plan_json: trace_context.context_plan_json.clone(),
            context_pack_refs_json: trace_context.context_pack_refs_json.clone(),
            usage,
            duration_ms,
            streamed,
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
