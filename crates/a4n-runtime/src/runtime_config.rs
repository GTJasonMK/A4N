use std::future::Future;

use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

tokio::task_local! {
    static RUNTIME_CONFIG_SNAPSHOT: RuntimeConfigSnapshot;
    static RUNTIME_USER_ID: i64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigSnapshot {
    #[serde(default, alias = "schema_version")]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub llm: Option<RuntimeLlmConfig>,
    #[serde(default, alias = "llm_configs")]
    pub llm_configs: Vec<RuntimeLlmConfig>,
    #[serde(default)]
    pub image: Option<RuntimeImageConfig>,
    #[serde(default, alias = "image_configs")]
    pub image_configs: Vec<RuntimeImageConfig>,
    #[serde(default)]
    pub video: Option<RuntimeVideoConfig>,
    #[serde(default, alias = "video_configs")]
    pub video_configs: Vec<RuntimeVideoConfig>,
    #[serde(default)]
    pub embedding: Option<RuntimeEmbeddingConfig>,
    #[serde(default, alias = "llm_default_source")]
    pub llm_default_source: Option<String>,
    #[serde(default, alias = "image_default_source")]
    pub image_default_source: Option<String>,
    #[serde(default, alias = "video_default_source")]
    pub video_default_source: Option<String>,
    #[serde(default, alias = "system_llm_api_key_encrypted")]
    pub system_llm_api_key_encrypted: Option<String>,
    #[serde(default, alias = "system_image_api_key_encrypted")]
    pub system_image_api_key_encrypted: Option<String>,
    #[serde(default, alias = "system_video_api_key_encrypted")]
    pub system_video_api_key_encrypted: Option<String>,
    #[serde(default, alias = "llm_max_tokens")]
    pub llm_max_tokens: Option<LlmMaxTokensConfig>,
    #[serde(default, alias = "client_context")]
    pub client_context: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLlmConfig {
    pub id: Option<i64>,
    pub name: String,
    pub provider: String,
    #[serde(alias = "base_url")]
    pub base_url: String,
    #[serde(default, alias = "api_key_encrypted")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    #[serde(alias = "max_concurrent")]
    pub max_concurrent: i32,
    #[serde(alias = "timeout_secs")]
    pub timeout_secs: i32,
    #[serde(alias = "max_context_tokens")]
    pub max_context_tokens: i32,
    #[serde(alias = "max_output_tokens")]
    pub max_output_tokens: i32,
    #[serde(default, alias = "request_options_json")]
    pub request_options_json: Option<Value>,
    #[serde(default, alias = "is_default")]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeImageConfig {
    pub id: Option<i64>,
    pub name: String,
    pub backend: String,
    #[serde(default, alias = "base_url")]
    pub base_url: Option<String>,
    #[serde(default, alias = "api_key_encrypted")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    #[serde(alias = "max_concurrent")]
    pub max_concurrent: i32,
    #[serde(alias = "default_width")]
    pub default_width: i32,
    #[serde(alias = "default_height")]
    pub default_height: i32,
    #[serde(default, alias = "is_default")]
    pub is_default: bool,
    #[serde(default, alias = "provider_options_json")]
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVideoConfig {
    pub id: Option<i64>,
    pub name: String,
    pub backend: String,
    #[serde(default, alias = "base_url")]
    pub base_url: Option<String>,
    #[serde(default, alias = "api_key_encrypted")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    #[serde(alias = "max_concurrent")]
    pub max_concurrent: i32,
    #[serde(alias = "default_width")]
    pub default_width: i32,
    #[serde(alias = "default_height")]
    pub default_height: i32,
    #[serde(alias = "default_duration_secs")]
    pub default_duration_secs: i32,
    #[serde(alias = "min_duration_secs")]
    pub min_duration_secs: i32,
    #[serde(alias = "max_duration_secs")]
    pub max_duration_secs: i32,
    #[serde(alias = "duration_mode")]
    pub duration_mode: String,
    #[serde(default, alias = "supported_durations_json")]
    pub supported_durations_json: Option<Value>,
    #[serde(alias = "default_fps")]
    pub default_fps: i32,
    #[serde(default, alias = "is_default")]
    pub is_default: bool,
    #[serde(default, alias = "provider_options_json")]
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEmbeddingConfig {
    pub id: Option<i64>,
    pub name: String,
    pub backend: String,
    #[serde(default, alias = "base_url")]
    pub base_url: Option<String>,
    #[serde(default, alias = "api_key_encrypted")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    #[serde(alias = "batch_size")]
    pub batch_size: i32,
    #[serde(default, alias = "local_model_path")]
    pub local_model_path: Option<String>,
    #[serde(default, alias = "is_default")]
    pub is_default: bool,
}

pub fn snapshot_from_args(args: &Value) -> Result<Option<RuntimeConfigSnapshot>, String> {
    let Some(value) = args.get("runtime_config_snapshot") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let snapshot: RuntimeConfigSnapshot = serde_json::from_value(value.clone())
        .map_err(|err| format!("runtime_config_snapshot 无效: {err}"))?;
    if let Some(version) = snapshot.schema_version
        && version != 1
    {
        return Err(format!(
            "runtime_config_snapshot.schema_version 不支持: {version}"
        ));
    }
    Ok(Some(snapshot))
}

pub async fn scope<F, T>(snapshot: RuntimeConfigSnapshot, future: F) -> T
where
    F: Future<Output = T>,
{
    RUNTIME_CONFIG_SNAPSHOT.scope(snapshot, future).await
}

pub async fn request_scope<F, T>(
    user_id: i64,
    snapshot: Option<RuntimeConfigSnapshot>,
    future: F,
) -> T
where
    F: Future<Output = T>,
{
    RUNTIME_USER_ID
        .scope(user_id, async move {
            match snapshot {
                Some(snapshot) => RUNTIME_CONFIG_SNAPSHOT.scope(snapshot, future).await,
                None => future.await,
            }
        })
        .await
}

pub fn current_snapshot() -> Option<RuntimeConfigSnapshot> {
    RUNTIME_CONFIG_SNAPSHOT.try_with(Clone::clone).ok()
}

pub fn current_user_id() -> Option<i64> {
    RUNTIME_USER_ID.try_with(|user_id| *user_id).ok()
}

pub fn current_llm_max_tokens() -> Option<LlmMaxTokensConfig> {
    current_snapshot().and_then(|snapshot| snapshot.llm_max_tokens)
}

pub fn uses_system_default_llm_template() -> bool {
    current_snapshot_default_source_is_system(|snapshot| snapshot.llm_default_source)
}

pub fn uses_system_default_image_template() -> bool {
    current_snapshot_default_source_is_system(|snapshot| snapshot.image_default_source)
}

pub fn uses_system_default_video_template() -> bool {
    current_snapshot_default_source_is_system(|snapshot| snapshot.video_default_source)
}

fn current_snapshot_default_source_is_system(
    source: impl FnOnce(RuntimeConfigSnapshot) -> Option<String>,
) -> bool {
    current_snapshot()
        .and_then(source)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("system"))
}

pub fn current_system_llm_api_key() -> Option<String> {
    current_snapshot_secret(|snapshot| snapshot.system_llm_api_key_encrypted)
}

pub fn current_system_image_api_key() -> Option<String> {
    current_snapshot_secret(|snapshot| snapshot.system_image_api_key_encrypted)
}

pub fn current_system_video_api_key() -> Option<String> {
    current_snapshot_secret(|snapshot| snapshot.system_video_api_key_encrypted)
}

fn current_snapshot_secret(
    secret: impl FnOnce(RuntimeConfigSnapshot) -> Option<String>,
) -> Option<String> {
    current_snapshot()
        .and_then(secret)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn current_llm_model() -> Option<llm_config::Model> {
    current_llm_model_for_id(None)
}

pub fn current_image_model() -> Option<image_gen_config::Model> {
    current_image_model_for_id(None)
}

pub fn current_video_model() -> Option<video_gen_config::Model> {
    current_video_model_for_id(None)
}

pub fn current_embedding_model() -> Option<embedding_config::Model> {
    current_snapshot()
        .and_then(|snapshot| snapshot.embedding)
        .map(embedding_model_from_runtime)
}

pub fn uses_client_desktop_embedding() -> bool {
    current_snapshot()
        .and_then(|snapshot| snapshot.client_context)
        .is_some_and(|context| {
            context
                .get("frontend_kind")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "tauri")
                && context
                    .get("backend_kind")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value == "remote_http")
        })
}

pub fn current_llm_model_for_id(config_id: Option<i64>) -> Option<llm_config::Model> {
    let snapshot = current_snapshot()?;
    match config_id {
        Some(id) => snapshot
            .llm
            .filter(|config| config.id == Some(id))
            .or_else(|| {
                snapshot
                    .llm_configs
                    .into_iter()
                    .find(|config| config.id == Some(id))
            })
            .map(llm_model_from_runtime),
        None => snapshot.llm.map(llm_model_from_runtime),
    }
}

pub fn current_image_model_for_id(config_id: Option<i64>) -> Option<image_gen_config::Model> {
    let snapshot = current_snapshot()?;
    match config_id {
        Some(id) => snapshot
            .image
            .filter(|config| config.id == Some(id))
            .or_else(|| {
                snapshot
                    .image_configs
                    .into_iter()
                    .find(|config| config.id == Some(id))
            })
            .map(image_model_from_runtime),
        None => snapshot.image.map(image_model_from_runtime),
    }
}

pub fn current_video_model_for_id(config_id: Option<i64>) -> Option<video_gen_config::Model> {
    let snapshot = current_snapshot()?;
    match config_id {
        Some(id) => snapshot
            .video
            .filter(|config| config.id == Some(id))
            .or_else(|| {
                snapshot
                    .video_configs
                    .into_iter()
                    .find(|config| config.id == Some(id))
            })
            .map(video_model_from_runtime),
        None => snapshot.video.map(video_model_from_runtime),
    }
}

pub fn current_llm_snapshot_scope_present() -> bool {
    current_snapshot()
        .map(|snapshot| snapshot.llm.is_some() || !snapshot.llm_configs.is_empty())
        .unwrap_or(false)
}

pub fn current_image_snapshot_scope_present() -> bool {
    current_snapshot()
        .map(|snapshot| snapshot.image.is_some() || !snapshot.image_configs.is_empty())
        .unwrap_or(false)
}

pub fn current_video_snapshot_scope_present() -> bool {
    current_snapshot()
        .map(|snapshot| snapshot.video.is_some() || !snapshot.video_configs.is_empty())
        .unwrap_or(false)
}

pub fn redacted_current_snapshot_value() -> Option<Value> {
    current_snapshot().map(|snapshot| redacted_snapshot_value(&snapshot))
}

pub fn redacted_snapshot_value(snapshot: &RuntimeConfigSnapshot) -> Value {
    let mut value = serde_json::to_value(snapshot).unwrap_or_else(|_| json!({}));
    redact_secret_fields(&mut value);
    value
}

fn redact_secret_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                let lower = key.to_ascii_lowercase();
                if lower.contains("api_key")
                    || lower.contains("apikey")
                    || lower.contains("token")
                    || lower.contains("secret")
                    || lower.contains("password")
                {
                    if !child.is_null() {
                        *child = Value::String("[REDACTED]".to_string());
                    }
                } else {
                    redact_secret_fields(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_secret_fields(item);
            }
        }
        _ => {}
    }
}

fn now() -> chrono::NaiveDateTime {
    Utc::now().naive_utc()
}

fn llm_model_from_runtime(cfg: RuntimeLlmConfig) -> llm_config::Model {
    let timestamp = now();
    llm_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        provider: cfg.provider,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        timeout_secs: cfg.timeout_secs,
        max_context_tokens: cfg.max_context_tokens,
        max_output_tokens: cfg.max_output_tokens,
        request_options_json: cfg.request_options_json,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}

fn image_model_from_runtime(cfg: RuntimeImageConfig) -> image_gen_config::Model {
    let timestamp = now();
    image_gen_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        default_width: cfg.default_width,
        default_height: cfg.default_height,
        provider_options_json: cfg.provider_options_json,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}

fn video_model_from_runtime(cfg: RuntimeVideoConfig) -> video_gen_config::Model {
    let timestamp = now();
    video_gen_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        default_width: cfg.default_width,
        default_height: cfg.default_height,
        default_duration_secs: cfg.default_duration_secs,
        min_duration_secs: cfg.min_duration_secs,
        max_duration_secs: cfg.max_duration_secs,
        duration_mode: cfg.duration_mode,
        supported_durations_json: cfg.supported_durations_json,
        default_fps: cfg.default_fps,
        provider_options_json: cfg.provider_options_json,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}

fn embedding_model_from_runtime(cfg: RuntimeEmbeddingConfig) -> embedding_config::Model {
    let timestamp = now();
    embedding_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
        model: cfg.model,
        dimension: cfg.dimension,
        batch_size: cfg.batch_size,
        local_model_path: cfg.local_model_path,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_llm_config(id: i64, is_default: bool) -> RuntimeLlmConfig {
        RuntimeLlmConfig {
            id: Some(id),
            name: format!("llm {id}"),
            provider: "openai".to_string(),
            base_url: "https://example.test/v1".to_string(),
            api_key_encrypted: Some("sk-test".to_string()),
            model: format!("gpt-test-{id}"),
            max_concurrent: 1,
            timeout_secs: 30,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            request_options_json: None,
            is_default,
        }
    }

    fn test_image_config(id: i64, is_default: bool) -> RuntimeImageConfig {
        RuntimeImageConfig {
            id: Some(id),
            name: format!("image {id}"),
            backend: "openai".to_string(),
            base_url: Some("https://example.test/v1".to_string()),
            api_key_encrypted: Some("sk-test".to_string()),
            model: format!("image-test-{id}"),
            max_concurrent: 1,
            default_width: 1024,
            default_height: 1024,
            is_default,
            provider_options_json: None,
        }
    }

    fn test_video_config(id: i64, is_default: bool) -> RuntimeVideoConfig {
        RuntimeVideoConfig {
            id: Some(id),
            name: format!("video {id}"),
            backend: "openai".to_string(),
            base_url: Some("https://example.test/v1".to_string()),
            api_key_encrypted: Some("sk-test".to_string()),
            model: format!("video-test-{id}"),
            max_concurrent: 1,
            default_width: 1280,
            default_height: 720,
            default_duration_secs: 5,
            min_duration_secs: 5,
            max_duration_secs: 5,
            duration_mode: "fixed".to_string(),
            supported_durations_json: None,
            default_fps: 24,
            is_default,
            provider_options_json: None,
        }
    }

    #[tokio::test]
    async fn explicit_config_id_lookup_accepts_active_or_list_config() {
        let snapshot = RuntimeConfigSnapshot {
            schema_version: Some(1),
            llm: Some(test_llm_config(1, true)),
            llm_configs: vec![test_llm_config(2, false)],
            image: Some(test_image_config(1, true)),
            image_configs: vec![test_image_config(2, false)],
            video: Some(test_video_config(1, true)),
            video_configs: vec![test_video_config(2, false)],
            embedding: None,
            llm_default_source: None,
            image_default_source: None,
            video_default_source: None,
            system_llm_api_key_encrypted: None,
            system_image_api_key_encrypted: None,
            system_video_api_key_encrypted: None,
            llm_max_tokens: None,
            client_context: None,
        };

        scope(snapshot, async {
            assert_eq!(current_llm_model().expect("default LLM").id, 1);
            assert_eq!(current_image_model().expect("default image").id, 1);
            assert_eq!(current_video_model().expect("default video").id, 1);

            assert_eq!(
                current_llm_model_for_id(Some(1))
                    .expect("active LLM by ID")
                    .id,
                1
            );
            assert_eq!(
                current_image_model_for_id(Some(1))
                    .expect("active image by ID")
                    .id,
                1
            );
            assert_eq!(
                current_video_model_for_id(Some(1))
                    .expect("active video by ID")
                    .id,
                1
            );

            assert_eq!(current_llm_model_for_id(Some(2)).expect("LLM by ID").id, 2);
            assert_eq!(
                current_image_model_for_id(Some(2)).expect("image by ID").id,
                2
            );
            assert_eq!(
                current_video_model_for_id(Some(2)).expect("video by ID").id,
                2
            );
        })
        .await;
    }

    #[tokio::test]
    async fn config_lists_do_not_define_runtime_defaults() {
        let snapshot = RuntimeConfigSnapshot {
            schema_version: Some(1),
            llm: None,
            llm_configs: vec![test_llm_config(2, true)],
            image: None,
            image_configs: vec![test_image_config(3, true)],
            video: None,
            video_configs: vec![test_video_config(4, true)],
            embedding: None,
            llm_default_source: None,
            image_default_source: None,
            video_default_source: None,
            system_llm_api_key_encrypted: None,
            system_image_api_key_encrypted: None,
            system_video_api_key_encrypted: None,
            llm_max_tokens: None,
            client_context: None,
        };

        scope(snapshot, async {
            assert!(current_llm_model().is_none());
            assert!(current_image_model().is_none());
            assert!(current_video_model().is_none());

            assert_eq!(current_llm_model_for_id(Some(2)).expect("LLM by ID").id, 2);
            assert_eq!(
                current_image_model_for_id(Some(3)).expect("image by ID").id,
                3
            );
            assert_eq!(
                current_video_model_for_id(Some(4)).expect("video by ID").id,
                4
            );
        })
        .await;
    }

    #[test]
    fn snapshot_from_args_rejects_malformed_snapshot() {
        let args = json!({
            "runtime_config_snapshot": {
                "schema_version": 1,
                "llm": {
                    "name": "local",
                    "provider": "openai"
                }
            }
        });

        let err = snapshot_from_args(&args).expect_err("malformed snapshot must fail explicitly");
        assert!(err.contains("runtime_config_snapshot 无效"));
    }

    #[test]
    fn snapshot_from_args_requires_runtime_llm_token_cap_fields() {
        let args = json!({
            "runtime_config_snapshot": {
                "schema_version": 1,
                "llm": {
                    "id": 1,
                    "name": "local",
                    "provider": "openai",
                    "base_url": "https://example.test/v1",
                    "api_key_encrypted": "sk-test",
                    "model": "gpt-test",
                    "max_concurrent": 1,
                    "timeout_secs": 30,
                    "is_default": true
                }
            }
        });

        let err = snapshot_from_args(&args).expect_err("runtime LLM caps are required");

        assert!(err.contains("runtime_config_snapshot 无效"));
        assert!(
            err.contains("maxContextTokens") || err.contains("max_context_tokens"),
            "{err}"
        );
    }

    #[test]
    fn redacted_snapshot_value_hides_secrets() {
        let snapshot = RuntimeConfigSnapshot {
            schema_version: Some(1),
            llm: Some(RuntimeLlmConfig {
                id: Some(1),
                name: "local llm".to_string(),
                provider: "openai".to_string(),
                base_url: "https://example.test".to_string(),
                api_key_encrypted: Some("sk-secret".to_string()),
                model: "gpt-test".to_string(),
                max_concurrent: 1,
                timeout_secs: 30,
                max_context_tokens: 128_000,
                max_output_tokens: 8_192,
                request_options_json: None,
                is_default: true,
            }),
            llm_configs: Vec::new(),
            image: None,
            image_configs: Vec::new(),
            video: None,
            video_configs: Vec::new(),
            embedding: None,
            llm_default_source: None,
            image_default_source: Some("system".to_string()),
            video_default_source: Some("system".to_string()),
            system_llm_api_key_encrypted: Some("sk-system-secret".to_string()),
            system_image_api_key_encrypted: Some("sk-image-secret".to_string()),
            system_video_api_key_encrypted: Some("sk-video-secret".to_string()),
            llm_max_tokens: None,
            client_context: Some(json!({
                "access_token": "token-secret",
                "safe": "visible"
            })),
        };

        let value = redacted_snapshot_value(&snapshot);
        assert_eq!(value["llm"]["apiKeyEncrypted"], "[REDACTED]");
        assert_eq!(value["systemLlmApiKeyEncrypted"], "[REDACTED]");
        assert_eq!(value["systemImageApiKeyEncrypted"], "[REDACTED]");
        assert_eq!(value["systemVideoApiKeyEncrypted"], "[REDACTED]");
        assert_eq!(value["clientContext"]["access_token"], "[REDACTED]");
        assert_eq!(value["clientContext"]["safe"], "visible");
    }
}
