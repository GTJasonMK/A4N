use a4n_error::AppError;
use a4n_llm_provider_contract::{ChatRequest, LlmProtocolKind};
use reqwest::{RequestBuilder, Url};
use serde_json::{Map, Value};

use super::HttpLlmProvider;
use crate::native::{gemini, responses};

impl HttpLlmProvider {
    pub(crate) fn native_request_body(
        &self,
        request: &ChatRequest,
    ) -> Result<serde_json::Value, AppError> {
        Self::native_request_body_with_config(
            self.protocol,
            &self.model,
            self.extra_body.clone().unwrap_or_default(),
            request,
        )
    }

    pub(crate) fn native_request_body_preview(
        protocol: LlmProtocolKind,
        model: &str,
        extra_body: Option<Value>,
        request: &ChatRequest,
    ) -> Result<Value, AppError> {
        let extra_body =
            a4n_llm_provider_contract::normalize_extra_body(extra_body)?.unwrap_or_default();
        Self::native_request_body_with_config(protocol, model, extra_body, request)
    }

    fn native_request_body_with_config(
        protocol: LlmProtocolKind,
        model: &str,
        extra_body: Map<String, Value>,
        request: &ChatRequest,
    ) -> Result<Value, AppError> {
        match protocol {
            LlmProtocolKind::OpenAiResponses => responses::request_body(request, model, extra_body),
            LlmProtocolKind::GeminiGenerateContent => gemini::request_body(request, extra_body),
            _ => Err(AppError::ConfigError("不支持的原生 LLM 协议".into())),
        }
    }

    pub(super) fn build_native_request(
        &self,
        request: &ChatRequest,
    ) -> Result<RequestBuilder, AppError> {
        let mut builder = match self.protocol {
            LlmProtocolKind::OpenAiResponses => {
                let body = self.native_request_body(request)?;
                let mut builder = self
                    .client
                    .post(format!("{}/responses", self.base_url))
                    .json(&body);
                if let Some(key) = &self.api_key {
                    builder = builder.bearer_auth(key);
                }
                builder
            }
            LlmProtocolKind::GeminiGenerateContent => {
                let mut url = Url::parse(&self.base_url)
                    .map_err(|error| AppError::ConfigError(format!("Gemini 地址无效: {error}")))?;
                let model = self.model.strip_prefix("models/").unwrap_or(&self.model);
                url.path_segments_mut()
                    .map_err(|_| AppError::ConfigError("Gemini 地址必须支持路径".into()))?
                    .pop_if_empty()
                    .push("models")
                    .push(&format!("{model}:streamGenerateContent"));
                url.query_pairs_mut().append_pair("alt", "sse");
                let body = self.native_request_body(request)?;
                let mut builder = self.client.post(url).json(&body);
                if let Some(key) = &self.api_key {
                    builder = builder.header("x-goog-api-key", key);
                }
                builder
            }
            _ => return Err(AppError::ConfigError("不支持的原生 LLM 协议".into())),
        };
        builder = builder.header("accept", "text/event-stream");
        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm_provider_contract::ModelCapabilities;

    #[test]
    fn native_requests_use_protocol_paths_auth_and_streaming_headers() {
        for (protocol, base, path, header) in [
            (
                LlmProtocolKind::OpenAiResponses,
                "https://proxy.example/v1",
                "/v1/responses",
                "authorization",
            ),
            (
                LlmProtocolKind::GeminiGenerateContent,
                "https://proxy.example/v1beta",
                "/v1beta/models/test:streamGenerateContent",
                "x-goog-api-key",
            ),
        ] {
            let provider = HttpLlmProvider::new_with_timeout_options_and_capabilities(
                base,
                "test",
                Some("fixture".into()),
                20,
                None,
                ModelCapabilities {
                    max_context_tokens: 128,
                    max_output_tokens: 32,
                },
            )
            .unwrap()
            .with_protocol(protocol);
            let request = provider
                .build_native_request(&ChatRequest::system_user("system", "ping", None, Some(8)))
                .unwrap()
                .build()
                .unwrap();
            assert_eq!(request.url().path(), path);
            assert!(request.headers().contains_key(header));
            assert_eq!(request.headers()["accept"], "text/event-stream");
            if protocol == LlmProtocolKind::GeminiGenerateContent {
                assert!(!request.headers().contains_key("authorization"));
                assert_eq!(request.url().query(), Some("alt=sse"));
            }
        }
    }
}
