use a4n_llm::provider::{LlmTraceContext, UsageInfo};
use a4n_orchestrator::{
    OrchestratorContext,
    generation::{LlmCallUsageTraceInput, RecordLlmCallTraceInput},
};

pub(in crate::dynamic_llm) async fn record_dynamic_llm_trace(
    orchestrator: &OrchestratorContext,
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
    let result = orchestrator
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
            usage: usage.map(llm_call_usage_trace_input),
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

fn llm_call_usage_trace_input(usage: UsageInfo) -> LlmCallUsageTraceInput {
    LlmCallUsageTraceInput {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
        prompt_cache_hit_tokens: usage.prompt_cache_hit_tokens,
        prompt_cache_miss_tokens: usage.prompt_cache_miss_tokens,
    }
}
