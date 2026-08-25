use super::*;

use super::aggregate::build_generation_call_analytics;
use sea_orm::{FromQueryResult, QuerySelect};
use serde_json::Value;

#[derive(Debug, FromQueryResult)]
struct LlmCallAnalyticsRow {
    id: i64,
    generation_run_id: Option<i64>,
    workflow_run_id: Option<i64>,
    call_key: Option<String>,
    input_fingerprint_sha256: Option<String>,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow: String,
    step_key: String,
    provider: Option<String>,
    model: Option<String>,
    context_plan_json: Option<Value>,
    prompt_tokens: Option<i32>,
    completion_tokens: Option<i32>,
    total_tokens: Option<i32>,
    prompt_cache_hit_tokens: Option<i32>,
    prompt_cache_miss_tokens: Option<i32>,
    prompt_cache_hit_ratio: Option<f64>,
    duration_ms: Option<i64>,
    streamed: bool,
    status: String,
    error_kind: Option<String>,
    error_message: Option<String>,
    finish_reason: Option<String>,
    created_at: chrono::NaiveDateTime,
}

impl From<LlmCallAnalyticsRow> for llm_call_trace::Model {
    fn from(row: LlmCallAnalyticsRow) -> Self {
        Self {
            id: row.id,
            generation_run_id: row.generation_run_id,
            workflow_run_id: row.workflow_run_id,
            call_key: row.call_key,
            input_fingerprint_sha256: row.input_fingerprint_sha256,
            project_id: row.project_id,
            root_project_id: row.root_project_id,
            workflow: row.workflow,
            step_key: row.step_key,
            provider: row.provider,
            model: row.model,
            context_plan_json: row.context_plan_json,
            context_pack_refs_json: None,
            prompt_tokens: row.prompt_tokens,
            completion_tokens: row.completion_tokens,
            total_tokens: row.total_tokens,
            prompt_cache_hit_tokens: row.prompt_cache_hit_tokens,
            prompt_cache_miss_tokens: row.prompt_cache_miss_tokens,
            prompt_cache_hit_ratio: row.prompt_cache_hit_ratio,
            duration_ms: row.duration_ms,
            streamed: row.streamed,
            status: row.status,
            error_kind: row.error_kind,
            error_message: row.error_message,
            finish_reason: row.finish_reason,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, FromQueryResult)]
struct GenerationRunAnalyticsRow {
    id: i64,
    project_id: i64,
    root_project_id: i64,
    workflow: String,
    step_key: String,
    title: String,
    model_kind: String,
    provider: Option<String>,
    model: Option<String>,
    status: String,
    workflow_run_id: Option<i64>,
    call_key: Option<String>,
    input_fingerprint_sha256: Option<String>,
    attempt: i32,
    error_message: Option<String>,
    created_at: chrono::NaiveDateTime,
    completed_at: Option<chrono::NaiveDateTime>,
}

impl From<GenerationRunAnalyticsRow> for generation_run::Model {
    fn from(row: GenerationRunAnalyticsRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            root_project_id: row.root_project_id,
            workflow: row.workflow,
            step_key: row.step_key,
            title: row.title,
            model_kind: row.model_kind,
            provider: row.provider,
            model: row.model,
            status: row.status,
            workflow_run_id: row.workflow_run_id,
            call_key: row.call_key,
            input_fingerprint_sha256: row.input_fingerprint_sha256,
            input_fingerprint_json: None,
            raw_response_json: None,
            parsed_output_json: None,
            validation_json: None,
            artifact_refs_json: None,
            attempt: row.attempt,
            prompt_template_names_json: None,
            prompt_template_hashes_json: None,
            rendered_messages_json: None,
            rendered_prompts_json: None,
            input_variables_json: None,
            source_refs_json: None,
            media_refs_json: None,
            request_json: None,
            response_summary_json: None,
            error_message: row.error_message,
            created_at: row.created_at,
            completed_at: row.completed_at,
        }
    }
}

pub async fn generation_call_analytics(
    db: &DatabaseConnection,
    input: GenerationCallAnalyticsInput,
) -> Result<GenerationCallAnalytics, AdminDbError> {
    let (days, recent_limit) = validate_generation_call_analytics_input(input)?;

    let now = Utc::now().naive_utc();
    let since = now - Duration::days(i64::from(days));

    let llm_calls = llm_call_trace::Entity::find()
        .select_only()
        .column(llm_call_trace::Column::Id)
        .column(llm_call_trace::Column::GenerationRunId)
        .column(llm_call_trace::Column::WorkflowRunId)
        .column(llm_call_trace::Column::CallKey)
        .column(llm_call_trace::Column::InputFingerprintSha256)
        .column(llm_call_trace::Column::ProjectId)
        .column(llm_call_trace::Column::RootProjectId)
        .column(llm_call_trace::Column::Workflow)
        .column(llm_call_trace::Column::StepKey)
        .column(llm_call_trace::Column::Provider)
        .column(llm_call_trace::Column::Model)
        .column(llm_call_trace::Column::ContextPlanJson)
        .column(llm_call_trace::Column::PromptTokens)
        .column(llm_call_trace::Column::CompletionTokens)
        .column(llm_call_trace::Column::TotalTokens)
        .column(llm_call_trace::Column::PromptCacheHitTokens)
        .column(llm_call_trace::Column::PromptCacheMissTokens)
        .column(llm_call_trace::Column::PromptCacheHitRatio)
        .column(llm_call_trace::Column::DurationMs)
        .column(llm_call_trace::Column::Streamed)
        .column(llm_call_trace::Column::Status)
        .column(llm_call_trace::Column::ErrorKind)
        .column(llm_call_trace::Column::ErrorMessage)
        .column(llm_call_trace::Column::FinishReason)
        .column(llm_call_trace::Column::CreatedAt)
        .filter(llm_call_trace::Column::CreatedAt.gte(since))
        .order_by_desc(llm_call_trace::Column::CreatedAt)
        .into_model::<LlmCallAnalyticsRow>()
        .all(db)
        .await
        .map_err(|source| AdminDbError::DatabaseOperation {
            operation: "读取 LLM 调用分析失败",
            source,
        })?
        .into_iter()
        .map(Into::into)
        .collect();

    let media_runs = generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::ProjectId)
        .column(generation_run::Column::RootProjectId)
        .column(generation_run::Column::Workflow)
        .column(generation_run::Column::StepKey)
        .column(generation_run::Column::Title)
        .column(generation_run::Column::ModelKind)
        .column(generation_run::Column::Provider)
        .column(generation_run::Column::Model)
        .column(generation_run::Column::Status)
        .column(generation_run::Column::WorkflowRunId)
        .column(generation_run::Column::CallKey)
        .column(generation_run::Column::InputFingerprintSha256)
        .column(generation_run::Column::Attempt)
        .column(generation_run::Column::ErrorMessage)
        .column(generation_run::Column::CreatedAt)
        .column(generation_run::Column::CompletedAt)
        .filter(generation_run::Column::CreatedAt.gte(since))
        .filter(generation_run::Column::ModelKind.is_in(["image", "video"]))
        .order_by_desc(generation_run::Column::CreatedAt)
        .into_model::<GenerationRunAnalyticsRow>()
        .all(db)
        .await
        .map_err(|source| AdminDbError::DatabaseOperation {
            operation: "读取媒体生成分析失败",
            source,
        })?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(build_generation_call_analytics(
        now,
        since,
        days,
        recent_limit,
        llm_calls,
        media_runs,
    ))
}

pub(super) fn validate_generation_call_analytics_input(
    input: GenerationCallAnalyticsInput,
) -> Result<(u32, u64), AdminDbError> {
    let days = input.days;
    if days == 0 || days > GENERATION_ANALYTICS_MAX_DAYS {
        return Err(AdminDbError::InvalidArgument(format!(
            "days 必须在 1..={GENERATION_ANALYTICS_MAX_DAYS} 之间"
        )));
    }
    let recent_limit = input.recent_limit;
    if recent_limit == 0 || recent_limit > GENERATION_ANALYTICS_MAX_RECENT_LIMIT {
        return Err(AdminDbError::InvalidArgument(format!(
            "recent_limit 必须在 1..={GENERATION_ANALYTICS_MAX_RECENT_LIMIT} 之间"
        )));
    }
    Ok((days, recent_limit))
}
