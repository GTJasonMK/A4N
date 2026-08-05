#[cfg(feature = "embedded-llm-provider")]
mod embedded_stream;

use super::*;

#[async_trait]
impl LlmProvider for DynamicLlmProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        if let Some(client) = self.sidecar_client.clone() {
            return self.chat_stream_via_sidecar(client, request).await;
        }

        #[cfg(not(feature = "embedded-llm-provider"))]
        {
            let _ = request;
            return Err(AppError::ConfigError(
                "当前 runtime 未编译 embedded LLM provider，无法执行 LLM stream；请启用 sidecar stream endpoint 或以 --features embedded-capabilities 显式构建 full profile"
                    .to_string(),
            ));
        }

        #[cfg(feature = "embedded-llm-provider")]
        {
            self.chat_stream_via_embedded(request).await
        }
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "ping".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: Some(0.0),
            max_tokens: Some(8),
            max_tokens_profile: None,
            response_format: None,
            stream_include_usage: false,
            cache_scope_key: None,
            cache_system_prefix: false,
            cache_user_prefix_len: None,
            trace_context: None,
            context_manifest_json: None,
            tools: None,
            tool_choice: None,
            stop_sequences: None,
        };
        if self.sidecar_client.is_some() {
            return self.probe_with_config_id(None, request).await;
        }
        a4n_llm::provider::collect_chat_stream(self, request)
            .await
            .map(|response| !response.message.content.trim().is_empty())
    }

    fn model_name(&self) -> &str {
        "dynamic-default-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            max_context_tokens: DEFAULT_LLM_MAX_CONTEXT_TOKENS,
            max_output_tokens: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        estimate_llm_tokens(text)
    }
}
