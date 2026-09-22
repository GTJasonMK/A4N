use super::{
    errors::{llm_response_shape_error, upstream_error_message},
    json::{optional_nested_usize, optional_usize, required_usize},
};
use a4n_error::AppError;
use a4n_llm_provider_contract::UsageInfo;
use serde_json::Value;

pub(crate) fn stream_usage_only_chunk(json: &Value) -> Result<Option<UsageInfo>, AppError> {
    if !json
        .get("choices")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        return Ok(None);
    }
    stream_usage(json)
}

pub(crate) fn stream_usage(json: &Value) -> Result<Option<UsageInfo>, AppError> {
    let Some(usage) = json.get("usage").filter(|usage| usage.is_object()) else {
        return Ok(None);
    };
    upstream_error_message(json)
        .is_none()
        .then(|| usage_info_from_json(usage))
        .transpose()
}

pub(crate) fn first_choice<'a>(
    json: &'a Value,
    field: &str,
    body: Option<&str>,
) -> Result<&'a Value, AppError> {
    let choices = json["choices"].as_array().ok_or_else(|| {
        llm_response_shape_error(json, format!("LLM 响应缺少数组字段 {field}"), body)
    })?;
    if choices.len() > 1
        || choices
            .first()
            .is_some_and(|choice| choice["index"].as_u64().is_some_and(|index| index != 0))
    {
        return Err(llm_response_shape_error(
            json,
            "LLM 单响应调用只接受 index=0 的一个候选".to_string(),
            body,
        ));
    }
    choices.first().ok_or_else(|| {
        llm_response_shape_error(json, format!("LLM 响应字段 {field} 不能为空"), body)
    })
}

pub(crate) fn usage_info_from_json(value: &Value) -> Result<UsageInfo, AppError> {
    let prompt_tokens = required_usize(&value["prompt_tokens"], "usage.prompt_tokens")?;
    let completion_tokens = required_usize(&value["completion_tokens"], "usage.completion_tokens")?;
    let total_tokens = required_usize(&value["total_tokens"], "usage.total_tokens")?;
    let prompt_cache_hit_tokens = prompt_cache_hit_tokens_from_usage(value)?;
    let prompt_cache_miss_tokens = optional_usize(
        value.get("prompt_cache_miss_tokens"),
        "usage.prompt_cache_miss_tokens",
    )?;
    let prompt_cache_write_tokens = match optional_nested_usize(
        value,
        &["prompt_tokens_details", "cache_write_tokens"],
        "usage.prompt_tokens_details.cache_write_tokens",
    )? {
        Some(tokens) => Some(tokens),
        None => optional_nested_usize(
            value,
            &["input_tokens_details", "cache_write_tokens"],
            "usage.input_tokens_details.cache_write_tokens",
        )?,
    };

    UsageInfo {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        prompt_cache_hit_tokens,
        prompt_cache_miss_tokens,
        prompt_cache_write_tokens,
    }
    .normalize_cache_tokens()
    .map_err(AppError::LlmError)
}

fn prompt_cache_hit_tokens_from_usage(value: &Value) -> Result<Option<usize>, AppError> {
    if let Some(tokens) = optional_usize(
        value.get("prompt_cache_hit_tokens"),
        "usage.prompt_cache_hit_tokens",
    )? {
        return Ok(Some(tokens));
    }
    if let Some(tokens) = optional_nested_usize(
        value,
        &["prompt_tokens_details", "cached_tokens"],
        "usage.prompt_tokens_details.cached_tokens",
    )? {
        return Ok(Some(tokens));
    }
    optional_nested_usize(
        value,
        &["input_tokens_details", "cached_tokens"],
        "usage.input_tokens_details.cached_tokens",
    )
}
