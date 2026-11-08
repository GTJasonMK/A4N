use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde::de::DeserializeOwned;
use tokio_util::sync::CancellationToken;

use crate::source_markers::SourceMarkerMap;

mod recovery;
pub(crate) use recovery::{correct_llm_response, structured_chat_response};

pub(crate) use a4n_llm_structured_stream::StructuredLlmProgress;
pub(crate) use a4n_llm_structured_stream::StructuredLlmStreamResult;

pub(crate) struct ChatJsonStreamInput<'a> {
    pub(crate) request: ChatRequest,
    pub(crate) prompt_name: &'a str,
    pub(crate) truncated_message: &'a str,
    pub(crate) structured_progress: Option<StructuredLlmProgress>,
    pub(crate) cancel_token: Option<&'a CancellationToken>,
    pub(crate) cancelled_message: &'a str,
}

pub(crate) fn stream_error_or_cancelled(
    error: String,
    cancel_token: Option<&CancellationToken>,
) -> AppError {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        AppError::Cancelled
    } else {
        AppError::LlmError(error)
    }
}

pub(crate) fn parse_strict_json_response<T>(raw: &str, context: &str) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let trimmed = raw.trim();
    let contract_hint = strict_json_contract_hint(trimmed);
    a4n_domain::json_contract::parse_strict_json_value(trimmed)
        .and_then(serde_json::from_value)
        .map_err(|err| {
        let parse_hint = strict_json_parse_error_hint(&err);
        AppError::InvalidArgument(format!(
            "{context} 响应必须是严格 JSON 对象{contract_hint}{parse_hint}: {err}; 输出预览：{}",
            json_response_preview(trimmed)
        ))
    })
}

pub(crate) fn parse_source_marked_json_response<T>(
    raw: &str,
    context: &str,
    markers: &SourceMarkerMap,
) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let value: serde_json::Value = parse_strict_json_response(raw, context)?;
    let value = markers.decode_output(&value)?;
    serde_json::from_value(value).map_err(AppError::Serialization)
}

pub(crate) fn ensure_checkpoint_parsed_output_matches<T: PartialEq + ?Sized>(
    checkpoint_parsed_output: &T,
    reparsed_output: &T,
    context: &str,
) -> Result<(), AppError> {
    if checkpoint_parsed_output != reparsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验{context} checkpoint"),
        });
    }
    Ok(())
}

pub(crate) fn restored_llm_stage_output<T, F>(
    checkpoint_parsed_output: &serde_json::Value,
    response_content: &str,
    checkpoint_key: &str,
    prompt_name: &str,
    validate: F,
) -> Result<(serde_json::Value, T), AppError>
where
    T: serde::Serialize,
    F: Fn(serde_json::Value) -> Result<T, AppError>,
{
    let reparsed = validate(parse_strict_json_response(response_content, prompt_name)?)?;
    let parsed_json = serde_json::to_value(&reparsed)?;
    ensure_checkpoint_parsed_output_matches(
        checkpoint_parsed_output,
        &parsed_json,
        checkpoint_key,
    )?;
    Ok((parsed_json, reparsed))
}

pub(crate) fn llm_response_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let prefix = trimmed.chars().take(180).collect::<String>();
    if trimmed.chars().count() <= 260 {
        return prefix;
    }
    let suffix = trimmed
        .chars()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix} … {suffix}")
}

fn strict_json_parse_error_hint(err: &serde_json::Error) -> &'static str {
    if matches!(err.classify(), serde_json::error::Category::Eof) {
        return "；响应在 JSON 闭合前结束，通常是模型输出被截断、连接提前结束或 max_tokens 不足";
    }
    ""
}

fn strict_json_contract_hint(text: &str) -> &'static str {
    let lower = text.trim_start().to_ascii_lowercase();
    if lower.starts_with("```") {
        return "；检测到 Markdown 代码围栏，严格 JSON 响应必须直接以 JSON 对象开始，不能用 ``` 或 ```json 包裹";
    }
    if lower.starts_with("<antml thinking>")
        || lower.starts_with("<thinking>")
        || lower.starts_with("<think>")
    {
        return "；检测到模型把 thinking/reasoning 标签写入了正文 content，说明当前 LLM/代理没有遵守严格 JSON response_format；请关闭该模型配置的思考输出、降低/禁用 reasoning，或改用支持 JSON mode 的 LLM 配置";
    }
    ""
}

fn json_response_preview(trimmed: &str) -> String {
    if trimmed.is_empty() {
        return "<空响应>".to_string();
    }

    let mut chars = trimmed.chars();
    let mut preview = chars.by_ref().take(240).collect::<String>();
    if chars.next().is_some() {
        preview.push_str("...");
    }
    preview
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_error_preserves_cancellation() {
        let token = CancellationToken::new();
        token.cancel();

        assert!(matches!(
            stream_error_or_cancelled("stopped".to_string(), Some(&token)),
            AppError::Cancelled
        ));
        assert!(matches!(
            stream_error_or_cancelled("transport".to_string(), None),
            AppError::LlmError(message) if message == "transport"
        ));
    }
}
