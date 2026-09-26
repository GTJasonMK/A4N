use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use a4n_domain::agent::{
    AGENT_PROTOCOL_JSON_COMPAT, AGENT_PROTOCOL_NATIVE_TOOL_CALLING, valid_agent_protocol,
};

pub use a4n_domain::cost::PricingPlan;
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};

use crate::serde_update::{deserialize_nullable_update_field, deserialize_required_nullable_field};

pub use a4n_domain::{ImageSize, VideoDurationMode};
pub use a4n_model_discovery_contract::{DiscoveredModel, ModelDiscoveryResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptRow {
    pub name: String,
    pub content: String,
}

/// Host adapter for settings import/export prompt side effects.
#[async_trait::async_trait]
pub trait SettingsAllHostPort: Send + Sync {
    async fn modified_prompts(&self) -> Result<Vec<PromptRow>, a4n_error::AppError>;

    async fn prepare_prompt_import(
        &self,
        rows: &[PromptRow],
    ) -> Result<Vec<(String, String)>, a4n_error::AppError>;

    async fn import_prompts(
        &self,
        rows: Vec<PromptRow>,
        overwrite: bool,
    ) -> Result<Vec<String>, a4n_error::AppError>;

    async fn rollback_prompts(
        &self,
        snapshots: Vec<(String, String)>,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing settings export boundary.
#[async_trait::async_trait]
pub trait SettingsExportApplicationPort: Send + Sync {
    async fn export_all_configs_to_json(
        &self,
        caller_user_id: i64,
        host: &dyn SettingsAllHostPort,
    ) -> Result<String, a4n_error::AppError>;
}

/// Host-facing settings import boundary.
#[async_trait::async_trait]
pub trait SettingsImportApplicationPort: Send + Sync {
    async fn import_all_configs(
        &self,
        caller_user_id: i64,
        host: &dyn SettingsAllHostPort,
        input: ImportAllConfigsInput,
    ) -> Result<Vec<String>, a4n_error::AppError>;
}

/// AI provider configuration read boundary.
///
/// Persistence, validation, admin authorization and API projection stay in
/// the configuration owner. Hosts only route typed requests through this
/// port and never handle configuration ORM models.
#[async_trait::async_trait]
pub trait ConfigReadApplicationPort: Send + Sync {
    async fn llm_configs_exist(&self) -> Result<bool, a4n_error::AppError>;

    async fn image_configs_exist(&self) -> Result<bool, a4n_error::AppError>;

    async fn video_configs_exist(&self) -> Result<bool, a4n_error::AppError>;

    async fn saved_api_key(
        &self,
        caller_user_id: i64,
        kind: ConfigProviderKind,
        config_id: i64,
    ) -> Result<Option<String>, a4n_error::AppError>;

    async fn llm_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<LlmProviderConfig, a4n_error::AppError>;

    async fn embedding_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<EmbeddingProviderConfig, a4n_error::AppError>;

    async fn image_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<ImageConfigForConnectivityTest, a4n_error::AppError>;

    async fn list_llm(&self) -> Result<Vec<LlmConfigSummary>, a4n_error::AppError>;

    async fn list_image(&self) -> Result<Vec<ImageConfigSummary>, a4n_error::AppError>;

    async fn list_video(&self) -> Result<Vec<VideoConfigSummary>, a4n_error::AppError>;

    async fn list_embedding(&self) -> Result<Vec<EmbeddingConfigSummary>, a4n_error::AppError>;
}

/// AI provider configuration mutation boundary.
///
/// Persistence, validation, admin authorization and API projection stay in
/// the configuration owner. Hosts only route typed requests through this
/// port and never handle configuration ORM models.
#[async_trait::async_trait]
pub trait ConfigMutationApplicationPort: Send + Sync {
    async fn insert_baseline_embedding_if_empty(&self) -> Result<bool, a4n_error::AppError>;

    async fn insert_baseline_llm_if_empty(
        &self,
        input: CreateLlmInput,
    ) -> Result<bool, a4n_error::AppError>;

    async fn insert_baseline_image_if_empty(
        &self,
        input: ImageConfigInput,
    ) -> Result<bool, a4n_error::AppError>;

    async fn insert_baseline_video_if_empty(
        &self,
        input: VideoConfigInput,
    ) -> Result<bool, a4n_error::AppError>;

    async fn create_llm(
        &self,
        caller_user_id: i64,
        input: CreateLlmInput,
    ) -> Result<LlmConfigSummary, a4n_error::AppError>;

    async fn update_llm(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<LlmConfigSummary, a4n_error::AppError>;

    async fn set_default_llm(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn delete_llm(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn create_image(
        &self,
        caller_user_id: i64,
        input: ImageConfigInput,
    ) -> Result<ImageConfigSummary, a4n_error::AppError>;

    async fn update_image(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<ImageConfigSummary, a4n_error::AppError>;

    async fn set_default_image(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn delete_image(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn create_video(
        &self,
        caller_user_id: i64,
        input: VideoConfigInput,
    ) -> Result<VideoConfigSummary, a4n_error::AppError>;

    async fn update_video(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<VideoConfigSummary, a4n_error::AppError>;

    async fn set_default_video(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn delete_video(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn create_embedding(
        &self,
        caller_user_id: i64,
        input: CreateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, a4n_error::AppError>;

    async fn update_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, a4n_error::AppError>;

    async fn set_default_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn delete_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), a4n_error::AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigProviderKind {
    Llm,
    Image,
    Video,
}

/// Image configuration needed by the connectivity-test provider path.
///
/// This is intentionally not serializable: the API port may return the
/// provider secret to Runtime, but it must not become a wire/API DTO.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageConfigForConnectivityTest {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub supported_sizes_json: Value,
    pub pricing: PricingPlan,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend(
    "required" = ["provider", "base_url", "config_id", "use_saved_api_key"]
))]
pub struct ModelDiscoveryInput {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable_field")]
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
    pub pricing: PricingPlan,
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
    #[serde(default)]
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
#[schemars(extend(
    "required" = [
        "name",
        "backend",
        "base_url",
        "model",
        "dimension",
        "batch_size",
        "local_model_path"
    ]
))]
pub struct UpdateEmbeddingInput {
    pub name: String,
    pub backend: String,
    #[serde(deserialize_with = "deserialize_required_nullable_field")]
    pub base_url: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    #[serde(deserialize_with = "deserialize_required_nullable_field")]
    pub local_model_path: Option<String>,
    #[serde(default)]
    pub pricing: PricingPlan,
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
    pub supported_sizes: Vec<ImageSize>,
    pub is_default: bool,
    pub pricing: PricingPlan,
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
    pub supported_sizes: Vec<ImageSize>,
    #[serde(default)]
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateImageConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub supported_sizes: Vec<ImageSize>,
    #[serde(default)]
    pub pricing: PricingPlan,
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
    #[serde(default = "default_agent_protocol")]
    pub agent_protocol: String,
    pub extra_body_json: Option<Value>,
    pub is_default: bool,
    pub pricing: PricingPlan,
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
    #[serde(default = "default_agent_protocol")]
    pub agent_protocol: String,
    pub extra_body_json: Option<Value>,
    #[serde(default)]
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
#[schemars(extend(
    "required" = [
        "name",
        "provider",
        "base_url",
        "model",
        "max_concurrent",
        "timeout_secs",
        "max_context_tokens",
        "max_output_tokens",
        "extra_body_json"
    ]
))]
pub struct UpdateLlmInput {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    #[serde(default = "default_agent_protocol")]
    pub agent_protocol: String,
    #[serde(deserialize_with = "deserialize_required_nullable_field")]
    pub extra_body_json: Option<Value>,
    #[serde(default)]
    pub pricing: PricingPlan,
}

fn default_agent_protocol() -> String {
    AGENT_PROTOCOL_NATIVE_TOOL_CALLING.to_string()
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
    pub pricing: PricingPlan,
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
    #[serde(default)]
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateVideoConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
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
    #[serde(default)]
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportAllConfigsInput {
    pub settings_json: String,
    pub overwrite: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        ImportAllConfigsInput, ModelDiscoveryInput, UpdateEmbeddingInput, UpdateImageConfigInput,
        UpdateLlmInput, UpdateVideoConfigInput, VideoConfigInput, VideoDurationMode,
    };
    use serde_json::{Value, json};

    fn assert_key_forwarding<T>(base: Value)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        for key in [None, Some(Value::Null), Some(json!("test-key"))] {
            let mut input = base.clone();
            if let Some(key) = key {
                input["api_key_encrypted"] = key;
            }
            let parsed: T = serde_json::from_value(input.clone()).unwrap();
            let forwarded = serde_json::to_value(&parsed).unwrap();
            assert_eq!(
                forwarded.get("api_key_encrypted"),
                input.get("api_key_encrypted")
            );
            assert_eq!(serde_json::from_value::<T>(forwarded).unwrap(), parsed);
        }
    }

    #[test]
    fn config_key_intent_survives_host_to_worker_serialization() {
        assert_key_forwarding::<UpdateLlmInput>(json!({
            "name": "llm", "provider": "openai", "base_url": "https://example.invalid/v1",
            "model": "model", "max_concurrent": 1, "timeout_secs": 30,
            "max_context_tokens": 4096, "max_output_tokens": 1024, "extra_body_json": null
        }));
        assert_key_forwarding::<UpdateImageConfigInput>(json!({
            "name": "image", "backend": "openai-compatible-image", "base_url": "https://example.invalid/v1",
            "model": "model", "max_concurrent": 1, "supported_sizes": [{"width": 1024, "height": 1024}]
        }));
        assert_key_forwarding::<UpdateVideoConfigInput>(json!({
            "name": "video", "backend": "openai-compatible-video", "base_url": "https://example.invalid/v1",
            "model": "model", "max_concurrent": 1, "default_width": 1280, "default_height": 720,
            "default_duration_secs": 5, "min_duration_secs": 1, "max_duration_secs": 60,
            "duration_mode": "range", "supported_durations_json": null, "default_fps": 24
        }));
        assert_key_forwarding::<UpdateEmbeddingInput>(json!({
            "name": "embedding", "backend": "api", "base_url": "https://example.invalid/v1",
            "model": "model", "dimension": 8, "batch_size": 2, "local_model_path": null
        }));
    }

    #[test]
    fn config_updates_require_complete_fields_and_preserve_key_intent() {
        let llm = json!({
            "name": "updated",
            "provider": "openai",
            "base_url": "https://example.invalid/v1",
            "model": "model",
            "max_concurrent": 1,
            "timeout_secs": 30,
            "max_context_tokens": 4096,
            "max_output_tokens": 1024,
            "extra_body_json": null
        });
        let input: UpdateLlmInput = serde_json::from_value(llm.clone()).unwrap();
        assert_eq!(input.api_key_encrypted, None);
        let mut clear_key = llm.clone();
        clear_key["api_key_encrypted"] = Value::Null;
        let input: UpdateLlmInput = serde_json::from_value(clear_key).unwrap();
        assert_eq!(input.api_key_encrypted, Some(None));
        let mut missing_name = llm;
        missing_name.as_object_mut().unwrap().remove("name");
        assert!(serde_json::from_value::<UpdateLlmInput>(missing_name).is_err());

        assert!(
            serde_json::from_value::<UpdateEmbeddingInput>(json!({
                "name": "embedding",
                "backend": "api",
                "base_url": null,
                "model": "model",
                "dimension": 1536,
                "batch_size": 32
            }))
            .is_err()
        );
    }

    #[test]
    fn full_config_import_contract_is_strict() {
        let input: ImportAllConfigsInput = serde_json::from_value(json!({
            "settings_json": "{}",
            "overwrite": false
        }))
        .unwrap();
        assert_eq!(input.settings_json, "{}");
        assert!(
            serde_json::from_value::<ImportAllConfigsInput>(json!({
                "settings_json": "{}",
                "overwrite": false,
                "legacy_mode": true
            }))
            .is_err()
        );
    }

    #[test]
    fn discovery_scope_fields_are_required() {
        let input: ModelDiscoveryInput = serde_json::from_value(json!({
            "provider": "openai",
            "base_url": "https://example.invalid",
            "api_key": null,
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
                "api_key": null,
                "use_saved_api_key": false
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ModelDiscoveryInput>(json!({
                "provider": "openai",
                "base_url": "https://example.invalid",
                "api_key": null,
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
