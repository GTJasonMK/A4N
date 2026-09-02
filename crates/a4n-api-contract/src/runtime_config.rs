use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_domain::ImageSize;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfigSnapshot {
    #[serde(deserialize_with = "deserialize_runtime_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub llm: Option<RuntimeLlmConfig>,
    #[serde(default)]
    pub llm_configs: Vec<RuntimeLlmConfig>,
    #[serde(default)]
    pub image: Option<RuntimeImageConfig>,
    #[serde(default)]
    pub image_configs: Vec<RuntimeImageConfig>,
    #[serde(default)]
    pub video: Option<RuntimeVideoConfig>,
    #[serde(default)]
    pub video_configs: Vec<RuntimeVideoConfig>,
    #[serde(default)]
    pub embedding: Option<RuntimeEmbeddingConfig>,
    #[serde(default)]
    pub llm_default_source: Option<String>,
    #[serde(default)]
    pub image_default_source: Option<String>,
    #[serde(default)]
    pub video_default_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_llm_api_key_encrypted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_image_api_key_encrypted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_video_api_key_encrypted: Option<String>,
    #[serde(default)]
    pub llm_max_tokens: Option<LlmMaxTokensConfig>,
    #[serde(default)]
    pub client_context: Option<Value>,
}

impl RuntimeConfigSnapshot {
    pub fn without_secrets(&self) -> Self {
        let mut snapshot = self.clone();
        for config in snapshot
            .llm
            .iter_mut()
            .chain(snapshot.llm_configs.iter_mut())
        {
            config.api_key_encrypted = None;
        }
        for config in snapshot
            .image
            .iter_mut()
            .chain(snapshot.image_configs.iter_mut())
        {
            config.api_key_encrypted = None;
        }
        for config in snapshot
            .video
            .iter_mut()
            .chain(snapshot.video_configs.iter_mut())
        {
            config.api_key_encrypted = None;
        }
        if let Some(config) = snapshot.embedding.as_mut() {
            config.api_key_encrypted = None;
        }
        snapshot.system_llm_api_key_encrypted = None;
        snapshot.system_image_api_key_encrypted = None;
        snapshot.system_video_api_key_encrypted = None;
        snapshot
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLlmConfig {
    #[serde(deserialize_with = "deserialize_positive_config_id")]
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    #[serde(default)]
    pub extra_body_json: Option<Value>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeImageConfig {
    #[serde(deserialize_with = "deserialize_positive_config_id")]
    pub id: i64,
    pub name: String,
    pub backend: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub supported_sizes: Vec<ImageSize>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeVideoConfig {
    #[serde(deserialize_with = "deserialize_positive_config_id")]
    pub id: i64,
    pub name: String,
    pub backend: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    #[serde(default)]
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEmbeddingConfig {
    #[serde(deserialize_with = "deserialize_positive_config_id")]
    pub id: i64,
    pub name: String,
    pub backend: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    #[serde(default)]
    pub local_model_path: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

fn deserialize_runtime_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let version = u32::deserialize(deserializer)?;
    if version != 1 {
        return Err(D::Error::custom(format!(
            "runtime_config_snapshot.schema_version 不支持: {version}"
        )));
    }
    Ok(version)
}

fn deserialize_positive_config_id<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let id = i64::deserialize(deserializer)?;
    if id <= 0 {
        return Err(D::Error::custom(format!(
            "运行时配置 ID 必须是正整数，实际为 {id}"
        )));
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::RuntimeConfigSnapshot;
    use serde_json::json;

    #[test]
    fn without_secrets_clears_active_list_and_system_keys() {
        let snapshot: RuntimeConfigSnapshot = serde_json::from_value(json!({
            "schema_version": 1,
            "llm": runtime_llm(1, "active"),
            "llm_configs": [runtime_llm(1, "list")],
            "image": runtime_image(2, "image"),
            "video": runtime_video(3, "video"),
            "embedding": runtime_embedding(4, "embedding"),
            "system_llm_api_key_encrypted": "system-llm",
            "system_image_api_key_encrypted": "system-image",
            "system_video_api_key_encrypted": "system-video"
        }))
        .unwrap();

        let sanitized = serde_json::to_value(snapshot.without_secrets()).unwrap();
        for pointer in [
            "/llm/api_key_encrypted",
            "/llm_configs/0/api_key_encrypted",
            "/image/api_key_encrypted",
            "/video/api_key_encrypted",
            "/embedding/api_key_encrypted",
            "/system_llm_api_key_encrypted",
            "/system_image_api_key_encrypted",
            "/system_video_api_key_encrypted",
        ] {
            assert!(
                sanitized
                    .pointer(pointer)
                    .is_none_or(|value| value.is_null()),
                "{pointer}"
            );
        }
    }

    fn runtime_llm(id: i64, key: &str) -> serde_json::Value {
        json!({
            "id": id, "name": "llm", "provider": "openai", "base_url": "https://example.test/v1",
            "api_key_encrypted": key, "model": "model", "max_concurrent": 1,
            "timeout_secs": 30, "max_context_tokens": 4096, "max_output_tokens": 1024
        })
    }

    fn runtime_image(id: i64, key: &str) -> serde_json::Value {
        json!({
            "id": id, "name": "image", "backend": "openai", "base_url": "https://example.test/v1",
            "api_key_encrypted": key, "model": "model", "max_concurrent": 1,
            "supported_sizes": [{"width": 1024, "height": 1024}]
        })
    }

    fn runtime_video(id: i64, key: &str) -> serde_json::Value {
        json!({
            "id": id, "name": "video", "backend": "openai", "base_url": "https://example.test/v1",
            "api_key_encrypted": key, "model": "model", "max_concurrent": 1,
            "default_width": 1280, "default_height": 720, "default_duration_secs": 5,
            "min_duration_secs": 5, "max_duration_secs": 5, "duration_mode": "fixed",
            "supported_durations_json": [5], "default_fps": 24
        })
    }

    fn runtime_embedding(id: i64, key: &str) -> serde_json::Value {
        json!({
            "id": id, "name": "embedding", "backend": "remote", "base_url": "https://example.test/v1",
            "api_key_encrypted": key, "model": "model", "dimension": 384, "batch_size": 32
        })
    }
}
