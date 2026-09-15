//! Serialized response contracts for the LLM sidecar.
//!
//! Provider implementations, LLM domain types, and HTTP transport stay outside
//! this crate so clients can depend on the wire shape alone.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmGatewayProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub api_key: Option<String>,
    pub timeout_secs: u64,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub extra_body_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmGatewayPayload<T> {
    pub provider: LlmGatewayProviderConfig,
    pub request: T,
}

pub type LlmProbePayload<T> = LlmGatewayPayload<T>;

pub type LlmStreamUsage = LlmProbeUsage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LlmStreamChunk {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<LlmStreamToolCall>>,
    pub finish_reason: Option<String>,
    pub index: u32,
    pub usage: Option<LlmStreamUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LlmStreamToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LlmProbeOutput {
    pub response_preview: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub usage: Option<LlmProbeUsage>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LlmProbeUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub prompt_cache_hit_tokens: Option<usize>,
    pub prompt_cache_miss_tokens: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_write_tokens: Option<usize>,
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::{
        LlmGatewayPayload, LlmGatewayProviderConfig, LlmProbeOutput, LlmProbeUsage, LlmStreamChunk,
        LlmStreamToolCall,
    };
    use serde_json::{Value, json};

    fn usage() -> LlmProbeUsage {
        LlmProbeUsage {
            prompt_tokens: 2,
            completion_tokens: 3,
            total_tokens: 5,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
            prompt_cache_write_tokens: None,
        }
    }

    #[test]
    fn gateway_provider_config_rejects_unknown_fields() {
        let error = serde_json::from_value::<LlmGatewayProviderConfig>(json!({
            "provider": "openai",
            "base_url": "https://example.invalid/v1",
            "model": "test",
            "api_key": null,
            "timeout_secs": 30,
            "extra_body_json": null,
            "legacy_field": true
        }))
        .expect_err("unknown provider config fields must fail at the sidecar boundary");

        assert!(error.to_string().contains("legacy_field"));
    }

    #[test]
    fn gateway_provider_config_requires_nullable_fields_to_be_present() {
        for config in [
            json!({
                "provider": "openai",
                "base_url": "https://example.invalid/v1",
                "model": "test",
                "timeout_secs": 30,
                "extra_body_json": null
            }),
            json!({
                "provider": "openai",
                "base_url": "https://example.invalid/v1",
                "model": "test",
                "api_key": null,
                "timeout_secs": 30
            }),
        ] {
            serde_json::from_value::<LlmGatewayProviderConfig>(config)
                .expect_err("provider config must keep explicit nullable fields");
        }
    }

    #[test]
    fn gateway_payload_requires_request_and_rejects_unknown_fields() {
        let error = serde_json::from_value::<LlmGatewayPayload<Value>>(json!({
            "provider": {
                "provider": "openai",
                "base_url": "https://example.invalid/v1",
                "model": "test",
                "api_key": null,
                "timeout_secs": 30,
                "extra_body_json": null
            },
            "request": {},
            "legacy_field": true
        }))
        .expect_err("unknown gateway payload fields must fail at the sidecar boundary");
        assert!(error.to_string().contains("legacy_field"));

        serde_json::from_value::<LlmGatewayPayload<Value>>(json!({
            "provider": {
                "provider": "openai",
                "base_url": "https://example.invalid/v1",
                "model": "test",
                "api_key": null,
                "timeout_secs": 30,
                "extra_body_json": null
            }
        }))
        .expect_err("gateway payload must require request");
    }

    #[test]
    fn stream_chunk_rejects_unknown_fields_and_roundtrips_tool_calls() {
        let error = serde_json::from_value::<LlmStreamChunk>(json!({
            "content": "tool",
            "reasoning_content": null,
            "tool_calls": null,
            "finish_reason": null,
            "index": 0,
            "usage": null,
            "legacy_field": true
        }))
        .expect_err("unknown stream chunk fields must fail at the sidecar boundary");
        assert!(error.to_string().contains("legacy_field"));

        let chunk = LlmStreamChunk {
            content: None,
            reasoning_content: Some("reasoning".to_string()),
            tool_calls: Some(vec![LlmStreamToolCall {
                id: "call-1".to_string(),
                name: "lookup".to_string(),
                arguments: json!({"key": "value"}),
                stream_index: Some(0),
                provider_metadata: Some(json!({"signature": "opaque"})),
            }]),
            finish_reason: Some("tool_calls".to_string()),
            index: 1,
            usage: None,
        };
        let encoded = serde_json::to_value(&chunk).expect("stream chunk should serialize");
        let decoded: LlmStreamChunk =
            serde_json::from_value(encoded).expect("stream chunk should deserialize");
        assert_eq!(decoded, chunk);
    }

    #[test]
    fn probe_output_rejects_unknown_fields() {
        let error = serde_json::from_value::<LlmProbeOutput>(json!({
            "response_preview": "pong",
            "usage": null,
            "finish_reason": "stop",
            "legacy_field": true
        }))
        .expect_err("unknown probe output fields must fail at the sidecar boundary");

        assert!(error.to_string().contains("legacy_field"));
    }

    #[test]
    fn probe_output_requires_nullable_fields_to_be_present() {
        for output in [
            json!({
                "response_preview": "pong",
                "finish_reason": "stop"
            }),
            json!({
                "response_preview": "pong",
                "usage": null
            }),
        ] {
            serde_json::from_value::<LlmProbeOutput>(output)
                .expect_err("probe output must keep its explicit nullable fields");
        }
    }

    #[test]
    fn probe_output_roundtrips_usage_without_provider_types() {
        let output = LlmProbeOutput {
            response_preview: "pong".to_string(),
            usage: Some(usage()),
            finish_reason: Some("stop".to_string()),
        };

        let encoded = serde_json::to_value(&output).expect("probe output should serialize");
        let decoded: LlmProbeOutput =
            serde_json::from_value(encoded).expect("probe output should deserialize");

        assert_eq!(decoded, output);
    }
}
