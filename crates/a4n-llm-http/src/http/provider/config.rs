use a4n_domain::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use a4n_error::AppError;
use a4n_llm_provider_contract::{LlmProtocolKind, ModelCapabilities, normalize_extra_body};
use serde_json::Value;

use crate::http::errors::{
    build_http_client, normalized_connect_timeout_secs, normalized_timeout_secs,
};

use super::HttpLlmProvider;

impl HttpLlmProvider {
    /// 手动创建并指定非流式请求总超时。
    ///
    /// 流式请求不会使用总超时，因为 reqwest 的总超时会覆盖整个响应体读取，
    /// 长文本生成即使持续输出也可能在固定时间后被强制中断。
    pub(in crate::http) fn new_with_timeout(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout_secs: u64,
    ) -> Self {
        let timeout_secs = normalized_timeout_secs(timeout_secs);
        let connect_timeout_secs = normalized_connect_timeout_secs(timeout_secs);
        let client = build_http_client(timeout_secs, connect_timeout_secs);

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            model: model.into(),
            timeout_secs,
            connect_timeout_secs,
            extra_body: None,
            protocol: LlmProtocolKind::OpenAiChatCompletions,
            caps: ModelCapabilities {
                max_context_tokens: DEFAULT_LLM_MAX_CONTEXT_TOKENS,
                max_output_tokens: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            },
        }
    }

    /// 手动创建并追加 provider-specific JSON 请求字段。
    pub(in crate::http) fn new_with_timeout_and_options(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout_secs: u64,
        extra_body: Option<Value>,
    ) -> Result<Self, AppError> {
        let mut provider = Self::new_with_timeout(base_url, model, api_key, timeout_secs);
        provider.extra_body = normalize_extra_body(extra_body)?;
        Ok(provider)
    }

    pub(in crate::http) fn new_with_timeout_options_and_capabilities(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout_secs: u64,
        extra_body: Option<Value>,
        caps: ModelCapabilities,
    ) -> Result<Self, AppError> {
        let mut provider =
            Self::new_with_timeout_and_options(base_url, model, api_key, timeout_secs, extra_body)?;
        provider.caps = caps;
        Ok(provider)
    }
}

impl std::fmt::Debug for HttpLlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpLlmProvider")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .finish()
    }
}
