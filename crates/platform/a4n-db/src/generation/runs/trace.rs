use super::super::*;
use a4n_domain::llm::{LlmCallUsageTraceInput, RecordLlmCallTraceInput};

pub async fn record_llm_call_trace<C>(
    db: &C,
    input: RecordLlmCallTraceInput,
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    if let Some(generation_run_id) = input.generation_run_id {
        validate_positive_id(generation_run_id, "generation_run_id")?;
    }
    if let Some(workflow_run_id) = input.workflow_run_id {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
    }
    if let Some(project_id) = input.project_id {
        validate_positive_id(project_id, "project_id")?;
    }
    if let Some(root_project_id) = input.root_project_id {
        validate_positive_id(root_project_id, "root_project_id")?;
    }

    let usage_trace = usage_trace_fields(input.usage.as_ref())?;
    llm_call_trace::ActiveModel {
        generation_run_id: Set(input.generation_run_id),
        workflow_run_id: Set(input.workflow_run_id),
        call_key: Set(normalize_optional_owned_text(input.call_key)),
        input_fingerprint_sha256: Set(normalize_optional_owned_text(
            input.input_fingerprint_sha256,
        )),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        workflow: Set(required_owned_text(input.workflow, "workflow")?),
        step_key: Set(required_owned_text(input.step_key, "step_key")?),
        provider: Set(normalize_optional_owned_text(input.provider)),
        model: Set(normalize_optional_owned_text(input.model)),
        context_plan_json: Set(input.context_plan_json),
        context_pack_refs_json: Set(input.context_pack_refs_json),
        prompt_tokens: Set(usage_trace.prompt_tokens),
        completion_tokens: Set(usage_trace.completion_tokens),
        total_tokens: Set(usage_trace.total_tokens),
        prompt_cache_hit_tokens: Set(usage_trace.prompt_cache_hit_tokens),
        prompt_cache_miss_tokens: Set(usage_trace.prompt_cache_miss_tokens),
        prompt_cache_write_tokens: Set(usage_trace.prompt_cache_write_tokens),
        prompt_cache_hit_ratio: Set(usage_trace.prompt_cache_hit_ratio),
        duration_ms: Set(input.duration_ms),
        streamed: Set(input.streamed),
        status: Set(required_owned_text(input.status, "status")?),
        error_kind: Set(normalize_optional_owned_text(input.error_kind)),
        error_message: Set(normalize_optional_owned_text(input.error_message)),
        finish_reason: Set(normalize_optional_owned_text(input.finish_reason)),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map(|_| ())
    .map_err(GenerationDbError::Database)
}

#[derive(Debug, Default)]
pub(in crate::generation) struct UsageTraceFields {
    pub(in crate::generation) prompt_tokens: Option<i32>,
    pub(in crate::generation) completion_tokens: Option<i32>,
    pub(in crate::generation) total_tokens: Option<i32>,
    pub(in crate::generation) prompt_cache_hit_tokens: Option<i32>,
    pub(in crate::generation) prompt_cache_miss_tokens: Option<i32>,
    pub(in crate::generation) prompt_cache_write_tokens: Option<i32>,
    pub(in crate::generation) prompt_cache_hit_ratio: Option<f64>,
}

pub(in crate::generation) fn usage_trace_fields(
    usage: Option<&LlmCallUsageTraceInput>,
) -> Result<UsageTraceFields, GenerationDbError> {
    let Some(usage) = usage else {
        return Ok(UsageTraceFields::default());
    };
    let usage = usage
        .clone()
        .normalize_cache_tokens()
        .map_err(GenerationDbError::InvalidArgument)?;
    let prompt_cache_hit_ratio = match (
        usage.prompt_cache_hit_tokens,
        usage.prompt_cache_miss_tokens,
    ) {
        (Some(hit), Some(miss)) if hit + miss > 0 => Some(hit as f64 / (hit + miss) as f64),
        _ => None,
    };
    Ok(UsageTraceFields {
        prompt_tokens: Some(usize_to_i32(usage.prompt_tokens, "prompt_tokens")?),
        completion_tokens: Some(usize_to_i32(usage.completion_tokens, "completion_tokens")?),
        total_tokens: Some(usize_to_i32(usage.total_tokens, "total_tokens")?),
        prompt_cache_hit_tokens: usage
            .prompt_cache_hit_tokens
            .map(|value| usize_to_i32(value, "prompt_cache_hit_tokens"))
            .transpose()?,
        prompt_cache_miss_tokens: usage
            .prompt_cache_miss_tokens
            .map(|value| usize_to_i32(value, "prompt_cache_miss_tokens"))
            .transpose()?,
        prompt_cache_write_tokens: usage
            .prompt_cache_write_tokens
            .map(|value| usize_to_i32(value, "prompt_cache_write_tokens"))
            .transpose()?,
        prompt_cache_hit_ratio,
    })
}

fn usize_to_i32(value: usize, field: &str) -> Result<i32, GenerationDbError> {
    i32::try_from(value).map_err(|_| {
        GenerationDbError::InvalidArgument(format!("LLM usage 字段 {field} 超出 i32 范围"))
    })
}
