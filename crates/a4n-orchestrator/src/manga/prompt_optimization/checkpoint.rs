use a4n_core::error::AppError;
use a4n_llm::provider::is_truncated_finish_reason;
use serde::Deserialize;
use serde_json::Value;

use crate::project::required_text;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OptimizedMangaPromptFields {
    pub(super) image_prompt: String,
}

#[derive(Debug)]
pub(super) struct ValidatedMangaPromptOptimization {
    pub(super) parsed_image_prompt: String,
    pub(super) image_prompt: String,
}

pub(super) fn validate_manga_prompt_optimization(
    optimized: OptimizedMangaPromptFields,
) -> Result<ValidatedMangaPromptOptimization, AppError> {
    let image_prompt = required_text(&optimized.image_prompt, "优化结果 image_prompt")
        .map_err(AppError::InvalidArgument)?
        .to_string();
    Ok(ValidatedMangaPromptOptimization {
        parsed_image_prompt: optimized.image_prompt,
        image_prompt,
    })
}

pub(super) fn restored_manga_prompt_optimization_checkpoint(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<
    (
        ValidatedMangaPromptOptimization,
        Option<Value>,
        Option<String>,
    ),
    AppError,
> {
    checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.response_content 必须是字符串"))
        })?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| AppError::InvalidArgument(format!("{call_key}.parsed_output 缺失")))?;
    let checkpoint_parsed: OptimizedMangaPromptFields =
        serde_json::from_value(parsed_output.clone()).map_err(|err| {
            AppError::InvalidArgument(format!("{call_key}.parsed_output 字段不符合要求: {err}"))
        })?;
    let validated = validate_manga_prompt_optimization(checkpoint_parsed)?;
    let output = checkpoint_json
        .get("output")
        .ok_or_else(|| AppError::InvalidArgument(format!("{call_key}.output 缺失")))?;
    let checkpoint_output: OptimizedMangaPromptFields = serde_json::from_value(output.clone())
        .map_err(|err| {
            AppError::InvalidArgument(format!("{call_key}.output 字段不符合要求: {err}"))
        })?;
    if checkpoint_output.image_prompt != validated.image_prompt {
        return Err(AppError::InvalidArgument(format!(
            "{call_key}.output.image_prompt 必须等于 parsed_output.image_prompt trim 后文本"
        )));
    }
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(value.as_str().map(ToString::to_string).ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.finish_reason 必须是字符串或 null"))
        })?),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        )));
    }
    Ok((validated, usage, finish_reason))
}
