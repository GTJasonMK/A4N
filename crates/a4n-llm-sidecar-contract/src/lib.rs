//! Serialized response contracts for the LLM sidecar.
//!
//! Provider implementations, LLM domain types, and HTTP transport stay outside
//! this crate so clients can depend on the wire shape alone.

use serde::{Deserialize, Deserializer, Serialize};

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
    use super::{LlmProbeOutput, LlmProbeUsage};
    use serde_json::json;

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
