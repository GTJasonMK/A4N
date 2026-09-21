use super::*;

use super::metrics::{
    avg_duration_ms, avg_f64, avg_i64, cache_hit_ratio, llm_context_plan_metrics, media_duration_ms,
};

pub(super) fn call_kind_overview(kind: String, acc: CallKindAccumulator) -> CallKindOverview {
    CallKindOverview {
        kind,
        calls: acc.calls,
        success: acc.success,
        failed: acc.failed,
        pending: acc.pending,
        total_tokens: acc.total_tokens,
        prompt_tokens: acc.prompt_tokens,
        completion_tokens: acc.completion_tokens,
        prompt_cache_hit_tokens: acc.prompt_cache_hit_tokens,
        prompt_cache_miss_tokens: acc.prompt_cache_miss_tokens,
        prompt_cache_write_tokens: acc.prompt_cache_write_tokens,
        prompt_cache_hit_ratio: (!acc.cache_usage_incomplete)
            .then(|| cache_hit_ratio(acc.prompt_cache_hit_tokens, acc.prompt_tokens))
            .flatten(),
        avg_duration_ms: avg_duration_ms(acc.duration_total_ms, acc.duration_count),
    }
}

pub(super) fn sorted_llm_workflows(
    workflows: BTreeMap<(String, String), LlmWorkflowAccumulator>,
) -> Vec<LlmWorkflowMetric> {
    let mut rows = workflows
        .into_iter()
        .map(|((workflow, step_key), acc)| LlmWorkflowMetric {
            workflow,
            step_key,
            calls: acc.calls,
            success: acc.success,
            failed: acc.failed,
            pending: acc.pending,
            total_tokens: acc.total_tokens,
            prompt_tokens: acc.prompt_tokens,
            completion_tokens: acc.completion_tokens,
            prompt_cache_hit_tokens: acc.prompt_cache_hit_tokens,
            prompt_cache_miss_tokens: acc.prompt_cache_miss_tokens,
            prompt_cache_write_tokens: acc.prompt_cache_write_tokens,
            prompt_cache_hit_ratio: (!acc.cache_usage_incomplete)
                .then(|| cache_hit_ratio(acc.prompt_cache_hit_tokens, acc.prompt_tokens))
                .flatten(),
            avg_duration_ms: avg_duration_ms(acc.duration_total_ms, acc.duration_count),
            avg_stable_prefix_tokens: avg_i64(
                acc.stable_prefix_tokens_total,
                acc.context_plan_count,
            ),
            avg_dynamic_delta_tokens: avg_i64(
                acc.dynamic_delta_tokens_total,
                acc.context_plan_count,
            ),
            avg_estimated_input_tokens: avg_i64(
                acc.estimated_input_tokens_total,
                acc.context_plan_count,
            ),
            dropped_optional_pack_count: acc.dropped_optional_pack_count,
            context_over_budget_failure_count: acc.context_over_budget_failure_count,
            estimated_vs_actual_prompt_tokens_ratio: avg_f64(
                acc.estimated_actual_ratio_total,
                acc.estimated_actual_ratio_count,
            ),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.total_tokens
            .cmp(&a.total_tokens)
            .then(b.calls.cmp(&a.calls))
    });
    rows.truncate(20);
    rows
}

pub(super) fn sorted_media_workflows(
    workflows: BTreeMap<(String, String, String), MediaWorkflowAccumulator>,
) -> Vec<MediaWorkflowMetric> {
    let mut rows = workflows
        .into_iter()
        .map(|((kind, workflow, step_key), acc)| MediaWorkflowMetric {
            kind,
            workflow,
            step_key,
            calls: acc.calls,
            success: acc.success,
            failed: acc.failed,
            pending: acc.pending,
            avg_duration_ms: avg_duration_ms(acc.duration_total_ms, acc.duration_count),
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| std::cmp::Reverse(row.calls));
    rows.truncate(20);
    rows
}

pub(super) fn recent_llm_call(call: llm_call_trace::Model) -> RecentLlmCall {
    let context_metrics = llm_context_plan_metrics(&call);
    RecentLlmCall {
        id: call.id,
        created_at: call.created_at.to_string(),
        workflow: call.workflow,
        step_key: call.step_key,
        provider: call.provider,
        model: call.model,
        prompt_tokens: call.prompt_tokens,
        completion_tokens: call.completion_tokens,
        total_tokens: call.total_tokens,
        prompt_cache_hit_tokens: call.prompt_cache_hit_tokens,
        prompt_cache_miss_tokens: call.prompt_cache_miss_tokens,
        prompt_cache_write_tokens: call.prompt_cache_write_tokens,
        prompt_cache_hit_ratio: call.prompt_cache_hit_ratio,
        duration_ms: call.duration_ms,
        streamed: call.streamed,
        status: call.status,
        error_kind: call.error_kind,
        finish_reason: call.finish_reason,
        stable_prefix_tokens: context_metrics
            .as_ref()
            .map(|metrics| metrics.stable_prefix_tokens),
        dynamic_delta_tokens: context_metrics
            .as_ref()
            .map(|metrics| metrics.dynamic_delta_tokens),
        estimated_input_tokens: context_metrics
            .as_ref()
            .map(|metrics| metrics.estimated_input_tokens),
        dropped_optional_pack_count: context_metrics
            .map(|metrics| metrics.dropped_optional_pack_count)
            .unwrap_or(0),
    }
}

pub(super) fn recent_media_call(run: generation_run::Model) -> RecentMediaCall {
    let duration_ms = media_duration_ms(&run);
    RecentMediaCall {
        id: run.id,
        created_at: run.created_at.to_string(),
        completed_at: run.completed_at.map(|value| value.to_string()),
        kind: run.model_kind,
        workflow: run.workflow,
        step_key: run.step_key,
        title: run.title,
        provider: run.provider,
        model: run.model,
        status: run.status,
        duration_ms,
    }
}
