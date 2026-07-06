use a4n_core::models::generation::{generation_run, llm_call_trace};
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::AppState;
use crate::actions::admin_records::{AdminPermission, require_admin_permission};
use a4n_core::error::AppError;

const DEFAULT_DAYS: u32 = 30;
const MAX_DAYS: u32 = 180;
const DEFAULT_RECENT_LIMIT: u64 = 40;
const MAX_RECENT_LIMIT: u64 = 200;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationCallAnalyticsInput {
    pub days: Option<u32>,
    pub recent_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationCallAnalytics {
    pub generated_at: String,
    pub since_at: String,
    pub days: u32,
    pub overview: Vec<CallKindOverview>,
    pub daily: Vec<DailyCallMetric>,
    pub llm_by_workflow: Vec<LlmWorkflowMetric>,
    pub media_by_workflow: Vec<MediaWorkflowMetric>,
    pub recent_llm_calls: Vec<RecentLlmCall>,
    pub recent_media_calls: Vec<RecentMediaCall>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallKindOverview {
    pub kind: String,
    pub calls: i64,
    pub success: i64,
    pub failed: i64,
    pub pending: i64,
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub prompt_cache_hit_tokens: i64,
    pub prompt_cache_miss_tokens: i64,
    pub prompt_cache_hit_ratio: Option<f64>,
    pub avg_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DailyCallMetric {
    pub date: String,
    pub llm_calls: i64,
    pub image_calls: i64,
    pub video_calls: i64,
    pub llm_total_tokens: i64,
    pub llm_prompt_cache_hit_tokens: i64,
    pub llm_prompt_cache_miss_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmWorkflowMetric {
    pub workflow: String,
    pub step_key: String,
    pub calls: i64,
    pub success: i64,
    pub failed: i64,
    pub pending: i64,
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub prompt_cache_hit_tokens: i64,
    pub prompt_cache_miss_tokens: i64,
    pub prompt_cache_hit_ratio: Option<f64>,
    pub avg_duration_ms: Option<i64>,
    pub avg_stable_prefix_tokens: Option<i64>,
    pub avg_dynamic_delta_tokens: Option<i64>,
    pub avg_estimated_input_tokens: Option<i64>,
    pub dropped_optional_pack_count: i64,
    pub context_over_budget_failure_count: i64,
    pub estimated_vs_actual_prompt_tokens_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaWorkflowMetric {
    pub kind: String,
    pub workflow: String,
    pub step_key: String,
    pub calls: i64,
    pub success: i64,
    pub failed: i64,
    pub pending: i64,
    pub avg_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentLlmCall {
    pub id: i64,
    pub created_at: String,
    pub workflow: String,
    pub step_key: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub prompt_cache_hit_tokens: Option<i32>,
    pub prompt_cache_miss_tokens: Option<i32>,
    pub prompt_cache_hit_ratio: Option<f64>,
    pub duration_ms: Option<i64>,
    pub streamed: bool,
    pub status: String,
    pub error_kind: Option<String>,
    pub finish_reason: Option<String>,
    pub stable_prefix_tokens: Option<i64>,
    pub dynamic_delta_tokens: Option<i64>,
    pub estimated_input_tokens: Option<i64>,
    pub dropped_optional_pack_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentMediaCall {
    pub id: i64,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub kind: String,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub duration_ms: Option<i64>,
}

#[derive(Default)]
struct CallKindAccumulator {
    calls: i64,
    success: i64,
    failed: i64,
    pending: i64,
    total_tokens: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    prompt_cache_hit_tokens: i64,
    prompt_cache_miss_tokens: i64,
    duration_total_ms: i64,
    duration_count: i64,
}

#[derive(Default)]
struct LlmWorkflowAccumulator {
    calls: i64,
    success: i64,
    failed: i64,
    pending: i64,
    total_tokens: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    prompt_cache_hit_tokens: i64,
    prompt_cache_miss_tokens: i64,
    duration_total_ms: i64,
    duration_count: i64,
    stable_prefix_tokens_total: i64,
    dynamic_delta_tokens_total: i64,
    estimated_input_tokens_total: i64,
    context_plan_count: i64,
    dropped_optional_pack_count: i64,
    context_over_budget_failure_count: i64,
    estimated_actual_ratio_total: f64,
    estimated_actual_ratio_count: i64,
}

#[derive(Default)]
struct MediaWorkflowAccumulator {
    calls: i64,
    success: i64,
    failed: i64,
    pending: i64,
    duration_total_ms: i64,
    duration_count: i64,
}

async fn get_generation_call_analytics(
    state: &AppState,
    input: GenerationCallAnalyticsInput,
) -> Result<GenerationCallAnalytics, String> {
    let days = input.days.unwrap_or(DEFAULT_DAYS);
    if days == 0 || days > MAX_DAYS {
        return Err(format!("days 必须在 1..={MAX_DAYS} 之间"));
    }
    let recent_limit = input.recent_limit.unwrap_or(DEFAULT_RECENT_LIMIT);
    if recent_limit == 0 || recent_limit > MAX_RECENT_LIMIT {
        return Err(format!("recent_limit 必须在 1..={MAX_RECENT_LIMIT} 之间"));
    }

    let now = Utc::now().naive_utc();
    let since = now - Duration::days(i64::from(days));

    let llm_calls = llm_call_trace::Entity::find()
        .filter(llm_call_trace::Column::CreatedAt.gte(since))
        .order_by_desc(llm_call_trace::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("读取 LLM 调用分析失败: {err}"))?;

    let media_runs = generation_run::Entity::find()
        .filter(generation_run::Column::CreatedAt.gte(since))
        .filter(generation_run::Column::ModelKind.is_in(["image", "video"]))
        .order_by_desc(generation_run::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("读取媒体生成分析失败: {err}"))?;

    Ok(build_generation_call_analytics(
        now,
        since,
        days,
        recent_limit,
        llm_calls,
        media_runs,
    ))
}

pub async fn get_generation_call_analytics_for_admin(
    state: &AppState,
    caller_user_id: i64,
    input: GenerationCallAnalyticsInput,
) -> Result<GenerationCallAnalytics, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    get_generation_call_analytics(state, input)
        .await
        .map_err(AppError::InvalidArgument)
}

fn build_generation_call_analytics(
    now: chrono::NaiveDateTime,
    since: chrono::NaiveDateTime,
    days: u32,
    recent_limit: u64,
    llm_calls: Vec<llm_call_trace::Model>,
    media_runs: Vec<generation_run::Model>,
) -> GenerationCallAnalytics {
    let mut overview = BTreeMap::<String, CallKindAccumulator>::new();
    overview.entry("llm".to_string()).or_default();
    overview.entry("image".to_string()).or_default();
    overview.entry("video".to_string()).or_default();

    let mut daily = BTreeMap::<String, DailyCallMetric>::new();
    let mut llm_by_workflow = BTreeMap::<(String, String), LlmWorkflowAccumulator>::new();
    let mut media_by_workflow =
        BTreeMap::<(String, String, String), MediaWorkflowAccumulator>::new();

    for call in &llm_calls {
        let context_metrics = llm_context_plan_metrics(call);
        let day = call.created_at.date().format("%Y-%m-%d").to_string();
        let daily_entry = daily.entry(day.clone()).or_insert_with(|| DailyCallMetric {
            date: day,
            ..Default::default()
        });
        daily_entry.llm_calls += 1;
        daily_entry.llm_total_tokens += i64_from_i32(call.total_tokens);
        daily_entry.llm_prompt_cache_hit_tokens += i64_from_i32(call.prompt_cache_hit_tokens);
        daily_entry.llm_prompt_cache_miss_tokens += i64_from_i32(call.prompt_cache_miss_tokens);

        let overview_entry = overview.entry("llm".to_string()).or_default();
        overview_entry.calls += 1;
        add_llm_status(overview_entry, &call.status);
        overview_entry.total_tokens += i64_from_i32(call.total_tokens);
        overview_entry.prompt_tokens += i64_from_i32(call.prompt_tokens);
        overview_entry.completion_tokens += i64_from_i32(call.completion_tokens);
        overview_entry.prompt_cache_hit_tokens += i64_from_i32(call.prompt_cache_hit_tokens);
        overview_entry.prompt_cache_miss_tokens += i64_from_i32(call.prompt_cache_miss_tokens);
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
        let day = run.created_at.date().format("%Y-%m-%d").to_string();
        let daily_entry = daily.entry(day.clone()).or_insert_with(|| DailyCallMetric {
            date: day,
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

fn i64_from_i32(value: Option<i32>) -> i64 {
    i64::from(value.unwrap_or(0))
}

fn add_duration(acc: &mut CallKindAccumulator, duration_ms: Option<i64>) {
    if let Some(duration_ms) = duration_ms {
        acc.duration_total_ms += duration_ms;
        acc.duration_count += 1;
    }
}

fn add_media_status(acc: &mut CallKindAccumulator, status: &str) {
    acc.calls += 1;
    match media_status_bucket(status) {
        MediaStatusBucket::Success => acc.success += 1,
        MediaStatusBucket::Failed => acc.failed += 1,
        MediaStatusBucket::Pending => acc.pending += 1,
    }
}

fn add_llm_status(acc: &mut CallKindAccumulator, status: &str) {
    match llm_status_bucket(status) {
        MediaStatusBucket::Success => acc.success += 1,
        MediaStatusBucket::Failed => acc.failed += 1,
        MediaStatusBucket::Pending => acc.pending += 1,
    }
}

enum MediaStatusBucket {
    Success,
    Failed,
    Pending,
}

fn media_status_bucket(status: &str) -> MediaStatusBucket {
    match status {
        "completed" | "success" | "succeeded" => MediaStatusBucket::Success,
        "failed" | "error" | "cancelled" | "canceled" => MediaStatusBucket::Failed,
        _ => MediaStatusBucket::Pending,
    }
}

fn llm_status_bucket(status: &str) -> MediaStatusBucket {
    match status {
        "completed" | "success" | "succeeded" => MediaStatusBucket::Success,
        "failed" | "error" | "cancelled" | "canceled" | "timeout" => MediaStatusBucket::Failed,
        _ => MediaStatusBucket::Pending,
    }
}

fn media_duration_ms(run: &generation_run::Model) -> Option<i64> {
    let completed_at = run.completed_at?;
    let duration = completed_at - run.created_at;
    duration.num_milliseconds().max(0).into()
}

fn avg_duration_ms(total: i64, count: i64) -> Option<i64> {
    if count > 0 { Some(total / count) } else { None }
}

fn avg_i64(total: i64, count: i64) -> Option<i64> {
    if count > 0 { Some(total / count) } else { None }
}

fn avg_f64(total: f64, count: i64) -> Option<f64> {
    if count > 0 {
        Some(total / count as f64)
    } else {
        None
    }
}

fn cache_hit_ratio(hit: i64, miss: i64) -> Option<f64> {
    let total = hit + miss;
    if total > 0 {
        Some(hit as f64 / total as f64)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
struct LlmContextPlanMetrics {
    stable_prefix_tokens: i64,
    dynamic_delta_tokens: i64,
    estimated_input_tokens: i64,
    dropped_optional_pack_count: i64,
}

fn llm_context_plan_metrics(call: &llm_call_trace::Model) -> Option<LlmContextPlanMetrics> {
    let plan = call.context_plan_json.as_ref()?.as_object()?;
    Some(LlmContextPlanMetrics {
        stable_prefix_tokens: json_i64_field(plan, "stable_prefix_tokens")?,
        dynamic_delta_tokens: json_i64_field(plan, "dynamic_delta_tokens")?,
        estimated_input_tokens: json_i64_field(plan, "estimated_input_tokens")?,
        dropped_optional_pack_count: plan
            .get("dropped_optional_pack_refs")
            .and_then(serde_json::Value::as_array)
            .map_or(0, |items| items.len() as i64),
    })
}

fn json_i64_field(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<i64> {
    object.get(key)?.as_i64()
}

fn is_context_over_budget_failure(call: &llm_call_trace::Model) -> bool {
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

fn call_kind_overview(kind: String, acc: CallKindAccumulator) -> CallKindOverview {
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
        prompt_cache_hit_ratio: cache_hit_ratio(
            acc.prompt_cache_hit_tokens,
            acc.prompt_cache_miss_tokens,
        ),
        avg_duration_ms: avg_duration_ms(acc.duration_total_ms, acc.duration_count),
    }
}

fn sorted_llm_workflows(
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
            prompt_cache_hit_ratio: cache_hit_ratio(
                acc.prompt_cache_hit_tokens,
                acc.prompt_cache_miss_tokens,
            ),
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

fn sorted_media_workflows(
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

fn recent_llm_call(call: llm_call_trace::Model) -> RecentLlmCall {
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

fn recent_media_call(run: generation_run::Model) -> RecentMediaCall {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn llm_call(id: i64, workflow: &str, total: i32, hit: i32, miss: i32) -> llm_call_trace::Model {
        llm_call_trace::Model {
            id,
            generation_run_id: None,
            workflow_run_id: None,
            call_key: None,
            input_fingerprint_sha256: None,
            project_id: None,
            root_project_id: None,
            workflow: workflow.to_string(),
            step_key: "step".to_string(),
            provider: Some("deepseek".to_string()),
            model: Some("deepseek-chat".to_string()),
            context_plan_json: Some(json!({})),
            context_pack_refs_json: None,
            prompt_tokens: Some(total - 10),
            completion_tokens: Some(10),
            total_tokens: Some(total),
            prompt_cache_hit_tokens: Some(hit),
            prompt_cache_miss_tokens: Some(miss),
            prompt_cache_hit_ratio: None,
            duration_ms: Some(1000),
            streamed: false,
            status: "completed".to_string(),
            error_kind: None,
            error_message: None,
            finish_reason: Some("stop".to_string()),
            created_at: Utc::now().naive_utc(),
        }
    }

    fn llm_call_with_context_plan(
        id: i64,
        workflow: &str,
        status: &str,
        prompt_tokens: i32,
        plan: serde_json::Value,
    ) -> llm_call_trace::Model {
        let mut call = llm_call(id, workflow, prompt_tokens + 10, 0, 0);
        call.status = status.to_string();
        call.prompt_tokens = Some(prompt_tokens);
        call.context_plan_json = Some(plan);
        call
    }

    #[test]
    fn analytics_aggregates_llm_cache_ratio() {
        let now = Utc::now().naive_utc();
        let result = build_generation_call_analytics(
            now,
            now - Duration::days(30),
            30,
            10,
            vec![
                llm_call(1, "video_prompt", 100, 80, 20),
                llm_call(2, "video_prompt", 50, 20, 30),
            ],
            vec![],
        );
        let llm = result
            .overview
            .iter()
            .find(|row| row.kind == "llm")
            .expect("llm overview should exist");
        assert_eq!(llm.calls, 2);
        assert_eq!(llm.total_tokens, 150);
        assert_eq!(llm.prompt_cache_hit_tokens, 100);
        assert_eq!(llm.prompt_cache_miss_tokens, 50);
        assert_eq!(llm.prompt_cache_hit_ratio, Some(100.0 / 150.0));
    }

    #[test]
    fn analytics_aggregates_llm_context_quality_metrics() {
        let now = Utc::now().naive_utc();
        let mut failed = llm_call_with_context_plan(
            2,
            "video_prompt",
            "failed",
            50,
            json!({
                "stable_prefix_tokens": 80,
                "dynamic_delta_tokens": 20,
                "estimated_input_tokens": 100,
                "dropped_optional_pack_refs": [{ "pack_kind": "optional_a" }]
            }),
        );
        failed.error_kind = Some("context_budget".to_string());
        failed.error_message = Some("当前任务动态输入超过模型可用输入窗口".to_string());
        let result = build_generation_call_analytics(
            now,
            now - Duration::days(30),
            30,
            10,
            vec![
                llm_call_with_context_plan(
                    1,
                    "video_prompt",
                    "completed",
                    100,
                    json!({
                        "stable_prefix_tokens": 120,
                        "dynamic_delta_tokens": 40,
                        "estimated_input_tokens": 160,
                        "dropped_optional_pack_refs": []
                    }),
                ),
                failed,
            ],
            vec![],
        );
        let overview = result
            .overview
            .iter()
            .find(|row| row.kind == "llm")
            .expect("llm overview should exist");
        assert_eq!(overview.calls, 2);
        assert_eq!(overview.success, 1);
        assert_eq!(overview.failed, 1);

        let workflow = result
            .llm_by_workflow
            .iter()
            .find(|row| row.workflow == "video_prompt")
            .expect("workflow row should exist");
        assert_eq!(workflow.calls, 2);
        assert_eq!(workflow.success, 1);
        assert_eq!(workflow.failed, 1);
        assert_eq!(workflow.avg_stable_prefix_tokens, Some(100));
        assert_eq!(workflow.avg_dynamic_delta_tokens, Some(30));
        assert_eq!(workflow.avg_estimated_input_tokens, Some(130));
        assert_eq!(workflow.dropped_optional_pack_count, 1);
        assert_eq!(workflow.context_over_budget_failure_count, 1);
        assert_eq!(
            workflow.estimated_vs_actual_prompt_tokens_ratio,
            Some(((160.0 / 100.0) + (100.0 / 50.0)) / 2.0)
        );

        let recent = result
            .recent_llm_calls
            .iter()
            .find(|row| row.id == 2)
            .expect("failed call should be recent");
        assert_eq!(recent.status, "failed");
        assert_eq!(recent.stable_prefix_tokens, Some(80));
        assert_eq!(recent.dynamic_delta_tokens, Some(20));
        assert_eq!(recent.estimated_input_tokens, Some(100));
        assert_eq!(recent.dropped_optional_pack_count, 1);
    }
}
