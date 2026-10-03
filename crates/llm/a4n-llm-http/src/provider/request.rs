use std::time::Duration;

use a4n_error::AppError;
use a4n_llm_provider_contract::{
    ChatRequest, MessageRole, ModelCapabilities, normalize_extra_body,
};
use serde_json::{Map, Value, json};

use crate::cache::{apply_openai_cache_options, openai_content_with_images};
use crate::sse::enable_stream_usage;

use super::HttpLlmProvider;

#[derive(serde::Serialize)]
struct OpenAiMessage {
    role: String,
    content: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

fn insert_optional<T: serde::Serialize>(
    body: &mut Map<String, Value>,
    key: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        body.insert(key.to_string(), json!(value));
    }
}

impl HttpLlmProvider {
    fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// 构建请求。
    pub(crate) fn request_body(
        &self,
        request: &ChatRequest,
        stream: bool,
    ) -> Result<Value, AppError> {
        Self::request_body_with_config(
            &self.model,
            self.extra_body.as_ref(),
            &self.caps,
            self.replay_reasoning,
            request,
            stream,
        )
    }

    pub(crate) fn request_body_preview(
        model: &str,
        extra_body: Option<Value>,
        capabilities: ModelCapabilities,
        replay_reasoning: bool,
        request: &ChatRequest,
    ) -> Result<Value, AppError> {
        let extra_body = normalize_extra_body(extra_body)?;
        Self::request_body_with_config(
            model,
            extra_body.as_ref(),
            &capabilities,
            replay_reasoning,
            request,
            true,
        )
    }

    fn request_body_with_config(
        model: &str,
        extra_body: Option<&Map<String, Value>>,
        capabilities: &ModelCapabilities,
        replay_reasoning: bool,
        request: &ChatRequest,
        stream: bool,
    ) -> Result<Value, AppError> {
        let messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                Ok(OpenAiMessage {
                    role: match message.role {
                        MessageRole::System => "system".into(),
                        MessageRole::User => "user".into(),
                        MessageRole::Assistant => "assistant".into(),
                        MessageRole::Tool => "tool".into(),
                    },
                    content: openai_content_with_images(request, index, "text", "image_url")?,
                    tool_calls: message.tool_calls.as_ref().map(|tool_calls| {
                        tool_calls
                            .iter()
                            .map(|tool_call| {
                                json!({
                                    "id": tool_call.id,
                                    "type": "function",
                                    "function": {
                                        "name": tool_call.name,
                                        "arguments": tool_call.arguments.to_string()
                                    }
                                })
                            })
                            .collect()
                    }),
                    tool_call_id: message.tool_call_id.clone(),
                    // Thinking-mode tools require the field even when the
                    // provider chose a tool without emitting any reasoning.
                    reasoning_content: (replay_reasoning
                        && message
                            .tool_calls
                            .as_ref()
                            .is_some_and(|calls| !calls.is_empty()))
                    .then(|| {
                        message
                            .tool_calls
                            .as_ref()
                            .and_then(|calls| calls.first())
                            .and_then(|call| call.provider_metadata.as_ref())
                            .and_then(|metadata| metadata.get("reasoning_content"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string()
                    }),
                })
            })
            .collect::<Result<_, AppError>>()?;

        let mut body = extra_body.cloned().unwrap_or_default();
        body.insert("model".into(), Value::String(model.to_string()));
        body.insert("messages".into(), json!(messages));
        body.insert("n".into(), json!(1));
        let model = model.rsplit('/').next().unwrap_or(model);
        let o_series = matches!(model.split('-').next(), Some("o1" | "o3" | "o4"));
        // o-series uses a completion budget including reasoning and does not
        // accept the generic workflow sampling temperature.
        if !o_series {
            insert_optional(&mut body, "temperature", request.temperature);
        }
        body.insert(
            if o_series {
                "max_completion_tokens"
            } else {
                "max_tokens"
            }
            .to_string(),
            json!(request.max_tokens.unwrap_or(capabilities.max_output_tokens)),
        );
        apply_openai_cache_options(request, &mut body)?;
        insert_optional(
            &mut body,
            "response_format",
            request.response_format.as_ref(),
        );
        insert_optional(&mut body, "tools", request.tools.as_ref());
        insert_optional(&mut body, "tool_choice", request.tool_choice.as_deref());
        insert_optional(&mut body, "stop", request.stop_sequences.as_ref());
        if stream {
            body.insert("stream".into(), Value::Bool(true));
            if request.stream_include_usage {
                enable_stream_usage(&mut body);
            }
        }

        Ok(Value::Object(body))
    }

    pub(crate) fn build_request(
        &self,
        request: &ChatRequest,
        stream: bool,
    ) -> Result<reqwest::RequestBuilder, AppError> {
        let body = self.request_body(request, stream)?;
        let mut request_builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body);

        if let Some(api_key) = &self.api_key {
            request_builder = request_builder.header("Authorization", format!("Bearer {api_key}"));
        }

        if !stream {
            request_builder = request_builder.timeout(self.request_timeout());
        }

        Ok(request_builder)
    }
}
