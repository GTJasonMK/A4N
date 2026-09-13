use super::*;

pub(super) fn i64_from_i32(value: Option<i32>) -> i64 {
    i64::from(value.unwrap_or(0))
}

pub(super) fn add_reported_tokens(total: &mut Option<i64>, value: Option<i32>) {
    if let Some(value) = value {
        *total = Some(total.unwrap_or(0) + i64::from(value));
    }
}

pub(super) fn add_duration(acc: &mut CallKindAccumulator, duration_ms: Option<i64>) {
    if let Some(duration_ms) = duration_ms {
        acc.duration_total_ms += duration_ms;
        acc.duration_count += 1;
    }
}

pub(super) fn add_media_status(acc: &mut CallKindAccumulator, status: &str) {
    acc.calls += 1;
    match media_status_bucket(status) {
        MediaStatusBucket::Success => acc.success += 1,
        MediaStatusBucket::Failed => acc.failed += 1,
        MediaStatusBucket::Pending => acc.pending += 1,
    }
}

pub(super) fn add_llm_status(acc: &mut CallKindAccumulator, status: &str) {
    match llm_status_bucket(status) {
        MediaStatusBucket::Success => acc.success += 1,
        MediaStatusBucket::Failed => acc.failed += 1,
        MediaStatusBucket::Pending => acc.pending += 1,
    }
}

pub(super) fn media_status_bucket(status: &str) -> MediaStatusBucket {
    match status {
        "completed" | "success" | "succeeded" => MediaStatusBucket::Success,
        "failed" | "error" | "cancelled" | "canceled" => MediaStatusBucket::Failed,
        _ => MediaStatusBucket::Pending,
    }
}

pub(super) fn llm_status_bucket(status: &str) -> MediaStatusBucket {
    match status {
        "completed" | "success" | "succeeded" => MediaStatusBucket::Success,
        "failed" | "error" | "cancelled" | "canceled" | "timeout" => MediaStatusBucket::Failed,
        _ => MediaStatusBucket::Pending,
    }
}

pub(super) fn media_duration_ms(run: &generation_run::Model) -> Option<i64> {
    let completed_at = run.completed_at?;
    let duration = completed_at - run.created_at;
    duration.num_milliseconds().max(0).into()
}

pub(super) fn avg_duration_ms(total: i64, count: i64) -> Option<i64> {
    if count > 0 { Some(total / count) } else { None }
}

pub(super) fn avg_i64(total: i64, count: i64) -> Option<i64> {
    if count > 0 { Some(total / count) } else { None }
}

pub(super) fn avg_f64(total: f64, count: i64) -> Option<f64> {
    if count > 0 {
        Some(total / count as f64)
    } else {
        None
    }
}

pub(super) fn cache_hit_ratio(hit: i64, input: i64) -> Option<f64> {
    if input > 0 && (0..=input).contains(&hit) {
        Some(hit as f64 / input as f64)
    } else {
        None
    }
}

pub(super) fn llm_context_plan_metrics(
    call: &llm_call_trace::Model,
) -> Option<LlmContextPlanMetrics> {
    let plan = call.context_plan_json.as_ref()?.as_object()?;
    Some(LlmContextPlanMetrics {
        stable_prefix_tokens: json_i64_field(plan, "stable_prefix_tokens")?,
        dynamic_delta_tokens: json_i64_field(plan, "dynamic_delta_tokens")?,
        estimated_input_tokens: json_i64_field(plan, "estimated_input_tokens")?,
        dropped_optional_pack_count: json_i64_field(plan, "dropped_optional_pack_count")?,
    })
}

fn json_i64_field(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<i64> {
    object.get(key)?.as_i64()
}

pub(super) fn is_context_over_budget_failure(call: &llm_call_trace::Model) -> bool {
    if !matches!(llm_status_bucket(&call.status), MediaStatusBucket::Failed) {
        return false;
    }
    let text = format!(
        "{}\n{}",
        call.error_kind.as_deref().unwrap_or_default(),
        call.error_message.as_deref().unwrap_or_default()
    )
    .to_lowercase();
    [
        "context",
        "budget",
        "token",
        "tokens",
        "上下文",
        "可用输入窗口",
        "放不下",
        "超出",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}
