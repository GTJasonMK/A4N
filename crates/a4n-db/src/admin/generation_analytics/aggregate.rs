use super::*;

use super::metrics::{
    add_duration, add_llm_status, add_media_status, add_reported_tokens, i64_from_i32,
    is_context_over_budget_failure, llm_context_plan_metrics, llm_status_bucket, media_duration_ms,
    media_status_bucket,
};
use super::projections::{
    call_kind_overview, recent_llm_call, recent_media_call, sorted_llm_workflows,
    sorted_media_workflows,
};

pub(super) fn build_generation_call_analytics(
    now: NaiveDateTime,
    since: NaiveDateTime,
    days: u32,
    recent_limit: u64,
    llm_calls: Vec<llm_call_trace::Model>,
    media_runs: Vec<generation_run::Model>,
) -> GenerationCallAnalytics {
    let mut overview = BTreeMap::<String, CallKindAccumulator>::new();
    overview.entry("llm".to_string()).or_default();
    overview.entry("image".to_string()).or_default();
    overview.entry("video".to_string()).or_default();

    let mut daily = BTreeMap::<NaiveDate, DailyCallMetric>::new();
    let mut llm_by_workflow = BTreeMap::<(String, String), LlmWorkflowAccumulator>::new();
    let mut media_by_workflow =
        BTreeMap::<(String, String, String), MediaWorkflowAccumulator>::new();

    for call in &llm_calls {
        let context_metrics = llm_context_plan_metrics(call);
        let cache_usage_incomplete = match call.prompt_tokens {
            Some(tokens) => tokens > 0 && call.prompt_cache_hit_tokens.is_none(),
            None => call.completion_tokens.is_some() || call.prompt_cache_hit_tokens.is_some(),
        };
        let day = call.created_at.date();
        let daily_entry = daily.entry(day).or_insert_with(|| DailyCallMetric {
            date: day.format("%Y-%m-%d").to_string(),
            ..Default::default()
        });
        daily_entry.llm_calls += 1;
        daily_entry.llm_total_tokens += i64_from_i32(call.total_tokens);
        daily_entry.llm_prompt_cache_hit_tokens += i64_from_i32(call.prompt_cache_hit_tokens);
        daily_entry.llm_prompt_cache_miss_tokens += i64_from_i32(call.prompt_cache_miss_tokens);
        add_reported_tokens(
            &mut daily_entry.llm_prompt_cache_write_tokens,
            call.prompt_cache_write_tokens,
        );

        let overview_entry = overview
            .get_mut("llm")
            .expect("llm overview accumulator initialized");
        overview_entry.calls += 1;
        add_llm_status(overview_entry, &call.status);
        overview_entry.total_tokens += i64_from_i32(call.total_tokens);
        overview_entry.prompt_tokens += i64_from_i32(call.prompt_tokens);
        overview_entry.completion_tokens += i64_from_i32(call.completion_tokens);
        overview_entry.prompt_cache_hit_tokens += i64_from_i32(call.prompt_cache_hit_tokens);
        overview_entry.prompt_cache_miss_tokens += i64_from_i32(call.prompt_cache_miss_tokens);
        add_reported_tokens(
            &mut overview_entry.prompt_cache_write_tokens,
            call.prompt_cache_write_tokens,
        );
        overview_entry.cache_usage_incomplete |= cache_usage_incomplete;
        add_duration(overview_entry, call.duration_ms);

        let workflow_key = (call.workflow.clone(), call.step_key.clone());
        let workflow_entry = llm_by_workflow.entry(workflow_key).or_default();
        workflow_entry.calls += 1;
        match llm_status_bucket(&call.status) {
            MediaStatusBucket::Success => workflow_entry.success += 1,
            MediaStatusBucket::Failed => workflow_entry.failed += 1,
            MediaStatusBucket::Pending => workflow_entry.pending += 1,
        }
        workflow_entry.total_tokens += i64_from_i32(call.total_tokens);
        workflow_entry.prompt_tokens += i64_from_i32(call.prompt_tokens);
        workflow_entry.completion_tokens += i64_from_i32(call.completion_tokens);
        workflow_entry.prompt_cache_hit_tokens += i64_from_i32(call.prompt_cache_hit_tokens);
        workflow_entry.prompt_cache_miss_tokens += i64_from_i32(call.prompt_cache_miss_tokens);
        add_reported_tokens(
            &mut workflow_entry.prompt_cache_write_tokens,
            call.prompt_cache_write_tokens,
        );
        workflow_entry.cache_usage_incomplete |= cache_usage_incomplete;
        if let Some(duration_ms) = call.duration_ms {
            workflow_entry.duration_total_ms += duration_ms;
            workflow_entry.duration_count += 1;
        }
        if let Some(metrics) = context_metrics {
            workflow_entry.stable_prefix_tokens_total += metrics.stable_prefix_tokens;
            workflow_entry.dynamic_delta_tokens_total += metrics.dynamic_delta_tokens;
            workflow_entry.estimated_input_tokens_total += metrics.estimated_input_tokens;
            workflow_entry.context_plan_count += 1;
            workflow_entry.dropped_optional_pack_count += metrics.dropped_optional_pack_count;
            if let Some(prompt_tokens) = call.prompt_tokens.filter(|value| *value > 0) {
                workflow_entry.estimated_actual_ratio_total +=
                    metrics.estimated_input_tokens as f64 / f64::from(prompt_tokens);
                workflow_entry.estimated_actual_ratio_count += 1;
            }
        }
        if is_context_over_budget_failure(call) {
            workflow_entry.context_over_budget_failure_count += 1;
        }
    }

    for run in &media_runs {
        let kind = run.model_kind.clone();
        let day = run.created_at.date();
        let daily_entry = daily.entry(day).or_insert_with(|| DailyCallMetric {
            date: day.format("%Y-%m-%d").to_string(),
            ..Default::default()
        });
        if kind == "image" {
            daily_entry.image_calls += 1;
        } else if kind == "video" {
            daily_entry.video_calls += 1;
        }

        let duration_ms = media_duration_ms(run);
        let overview_entry = overview.entry(kind.clone()).or_default();
        add_media_status(overview_entry, &run.status);
        add_duration(overview_entry, duration_ms);

        let workflow_key = (kind, run.workflow.clone(), run.step_key.clone());
        let workflow_entry = media_by_workflow.entry(workflow_key).or_default();
        workflow_entry.calls += 1;
        match media_status_bucket(&run.status) {
            MediaStatusBucket::Success => workflow_entry.success += 1,
            MediaStatusBucket::Failed => workflow_entry.failed += 1,
            MediaStatusBucket::Pending => workflow_entry.pending += 1,
        }
        if let Some(duration_ms) = duration_ms {
            workflow_entry.duration_total_ms += duration_ms;
            workflow_entry.duration_count += 1;
        }
    }

    let recent_limit = recent_limit as usize;
    GenerationCallAnalytics {
        generated_at: now.to_string(),
        since_at: since.to_string(),
        days,
        overview: overview
            .into_iter()
            .map(|(kind, acc)| call_kind_overview(kind, acc))
            .collect(),
        daily: daily.into_values().collect(),
        llm_by_workflow: sorted_llm_workflows(llm_by_workflow),
        media_by_workflow: sorted_media_workflows(media_by_workflow),
        recent_llm_calls: llm_calls
            .into_iter()
            .take(recent_limit)
            .map(recent_llm_call)
            .collect(),
        recent_media_calls: media_runs
            .into_iter()
            .take(recent_limit)
            .map(recent_media_call)
            .collect(),
    }
}
