use a4n_error::AppError;
use a4n_llm_provider_contract::{ChatChunk, LlmProvider, ModelCapabilities};
use a4n_llm_provider_contract::{ChatMessage, ChatRequest, ResponseFormat};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

pub(super) struct OutputFormatProvider {
    pub(super) provider: String,
    pub(super) inner: Box<dyn LlmProvider>,
}

#[async_trait]
impl LlmProvider for OutputFormatProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.inner
            .chat_stream(prepare_output_format(&self.provider, request)?)
            .await
    }
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }
    fn capabilities(&self) -> ModelCapabilities {
        self.inner.capabilities()
    }
    fn count_tokens(&self, text: &str) -> usize {
        self.inner.count_tokens(text)
    }
    fn count_messages_tokens(&self, messages: &[ChatMessage]) -> usize {
        self.inner.count_messages_tokens(messages)
    }
}

/// Compatibility is a declared protocol choice, never an HTTP-error fallback.
/// Explicit strict schemas are rejected if the preset cannot enforce them.
pub fn prepare_output_format(
    provider: &str,
    mut request: ChatRequest,
) -> Result<ChatRequest, AppError> {
    let Some(format) = request.response_format.as_ref() else {
        return Ok(request);
    };
    format.validate().map_err(AppError::InvalidArgument)?;
    if format.json_schema.is_none() {
        return Ok(request);
    }
    if matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "openai" | "openai-responses" | "custom-responses" | "gemini" | "custom-gemini"
    ) || (provider.trim().eq_ignore_ascii_case("anthropic")
        && format
            .json_schema
            .as_ref()
            .is_some_and(|schema| schema.strict))
    {
        return Ok(request);
    }
    let schema = format
        .json_schema
        .as_ref()
        .expect("validated schema format");
    if schema.strict {
        return Err(AppError::ConfigError(format!(
            "{provider} 预设未声明原生严格 Schema 能力，不能执行 strict=true 请求"
        )));
    }
    let instruction = format!(
        "[A4N 输出合同：提示词 Schema]\n此协议预设使用 JSON 模式及应用侧校验。输出必须遵守以下 JSON Schema，来源及业务规则仍以原任务为准：\n{}",
        schema.schema
    );
    if let Some(system) = request
        .messages
        .iter_mut()
        .find(|message| matches!(message.role, a4n_llm_provider_contract::MessageRole::System))
    {
        system.content.push_str("\n\n");
        system.content.push_str(&instruction);
    } else {
        request.messages.insert(0, ChatMessage::system(instruction));
        for breakpoint in &mut request.cache_breakpoints {
            breakpoint.message_index =
                breakpoint.message_index.checked_add(1).ok_or_else(|| {
                    AppError::InvalidArgument("输出 Schema 的缓存断点索引溢出".into())
                })?;
        }
    }
    request.response_format = Some(ResponseFormat::json_object());
    if let Some(trace) = request.trace_context.as_mut() {
        trace.context_plan_json = None;
        trace.context_pack_refs_json = None;
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schema_mode_is_explicit_and_never_silently_weakens_strict_requests() {
        let request = ChatRequest::system_user("system", "task", None, None)
            .with_json_schema_response_format(
                "test",
                json!({"type":"object","properties":{"id":{"type":"integer"}}}),
                false,
            )
            .unwrap();
        assert_eq!(
            prepare_output_format("openai-responses", request.clone())
                .unwrap()
                .response_format
                .unwrap()
                .r#type,
            "json_schema"
        );
        let prompt_only = prepare_output_format("deepseek", request.clone()).unwrap();
        assert_eq!(prompt_only.response_format.unwrap().r#type, "json_object");
        assert!(
            prompt_only
                .messages
                .first()
                .unwrap()
                .content
                .contains("提示词 Schema")
        );
        let mut strict = request;
        strict
            .response_format
            .as_mut()
            .unwrap()
            .json_schema
            .as_mut()
            .unwrap()
            .strict = true;
        assert!(prepare_output_format("deepseek", strict).is_err());
    }
}
