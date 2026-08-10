use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

pub use a4n_domain::{
    DiscoveredModel, LlmMaxTokensConfig, ModelDiscoveryResult, VideoDurationMode,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
#[schemars(extend(
    "required" = ["provider", "base_url", "config_id", "use_saved_api_key"]
))]
pub struct ModelDiscoveryInput {
    pub provider: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    #[serde(deserialize_with = "deserialize_required_config_id")]
    pub config_id: Option<i64>,
    pub use_saved_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EmbeddingConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub has_api_key: bool,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateEmbeddingInput {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateEmbeddingInput {
    pub name: Option<String>,
    pub backend: Option<String>,
    pub base_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: Option<String>,
    pub dimension: Option<i32>,
    pub batch_size: Option<i32>,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ImageConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateImageConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LlmConfigSummary {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub extra_body_json: Option<Value>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateLlmInput {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub extra_body_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateLlmInput {
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub provider: Option<String>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub base_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub api_key_encrypted: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub model: Option<String>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "i32")]
    pub max_concurrent: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "i32")]
    pub timeout_secs: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "i32")]
    pub max_context_tokens: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "i32")]
    pub max_output_tokens: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub extra_body_json: Option<Option<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VideoConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: VideoDurationMode,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: VideoDurationMode,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateVideoConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: VideoDurationMode,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
}

pub const USAGE_GUIDE_MAX_CHARS: usize = 100_000;
pub const USAGE_GUIDE_MAX_BLOCKS: usize = 200;
pub const USAGE_GUIDE_MAX_IMAGES: usize = 20;
pub const USAGE_GUIDE_IMAGE_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const USAGE_GUIDE_IMAGE_MAX_PIXELS: u64 = 40_000_000;
pub const USAGE_GUIDE_NEW_IMAGES_MAX_BYTES: usize = 20 * 1024 * 1024;
pub const USAGE_GUIDE_IMAGE_FILE_NAME_MAX_CHARS: usize = 255;
pub const USAGE_GUIDE_IMAGE_PATH_PREFIX: &str = "usage-guide/images/";
pub const USAGE_GUIDE_IMAGE_MAX_BASE64_CHARS: usize = USAGE_GUIDE_IMAGE_MAX_BYTES.div_ceil(3) * 4;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageGuideBlockKind {
    Text,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageGuideBlock {
    pub kind: UsageGuideBlockKind,
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    #[schemars(length(max = USAGE_GUIDE_IMAGE_MAX_BASE64_CHARS))]
    pub data_base64: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageGuideDocument {
    #[schemars(length(max = USAGE_GUIDE_MAX_BLOCKS))]
    pub blocks: Vec<UsageGuideBlock>,
}

fn deserialize_required_config_id<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<i64>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::{ModelDiscoveryInput, UpdateLlmInput, VideoConfigInput, VideoDurationMode};
    use serde_json::json;

    #[test]
    fn config_updates_preserve_null_semantics() {
        let input: UpdateLlmInput =
            serde_json::from_value(json!({"api_key_encrypted": null})).unwrap();
        assert_eq!(input.api_key_encrypted, Some(None));
        assert!(serde_json::from_value::<UpdateLlmInput>(json!({"name": null})).is_err());
    }

    #[test]
    fn discovery_scope_fields_are_required() {
        let input: ModelDiscoveryInput = serde_json::from_value(json!({
            "provider": "openai",
            "base_url": "https://example.invalid",
            "api_key_encrypted": null,
            "config_id": null,
            "use_saved_api_key": false
        }))
        .unwrap();
        assert_eq!(input.config_id, None);
        assert!(!input.use_saved_api_key);
        assert!(
            serde_json::from_value::<ModelDiscoveryInput>(json!({
                "provider": "openai",
                "base_url": "https://example.invalid",
                "api_key_encrypted": null,
                "use_saved_api_key": false
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ModelDiscoveryInput>(json!({
                "provider": "openai",
                "base_url": "https://example.invalid",
                "api_key_encrypted": null,
                "config_id": null
            }))
            .is_err()
        );
    }

    #[test]
    fn video_duration_mode_wire_contract_is_strict() {
        let value = json!({
            "name": "video",
            "backend": "openai-compatible-video",
            "base_url": "https://example.invalid/v1",
            "api_key_encrypted": null,
            "model": "video-model",
            "max_concurrent": 1,
            "default_width": 1280,
            "default_height": 720,
            "default_duration_secs": 5,
            "min_duration_secs": 1,
            "max_duration_secs": 60,
            "duration_mode": "range",
            "supported_durations_json": null,
            "default_fps": 24
        });
        let input: VideoConfigInput =
            serde_json::from_value(value.clone()).expect("canonical duration mode");
        assert_eq!(input.duration_mode, VideoDurationMode::Range);

        let mut noncanonical = value;
        noncanonical["duration_mode"] = json!(" fixed ");
        assert!(serde_json::from_value::<VideoConfigInput>(noncanonical).is_err());
    }
}
