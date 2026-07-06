use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ChatRequestStepWithBudgetInput, chat_request_step_with_budget,
    image_provider_request_step, model_request_preview, provider_request_step,
};
pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_core::models::ai::{
    embedding_config, image_gen_config, llm_config, system_settings, video_gen_config,
};
use a4n_engine::image::ImageGenerationRequest;
use a4n_engine::llm::provider::{ChatMessage, ChatRequest, LlmProvider};
use a4n_engine::queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_services::config::{
    CreateEmbeddingConfigParams, CreateImageConfigParams, UpdateEmbeddingConfigParams,
    UpdateImageConfigParams as ServiceUpdateImageConfigParams, validate_image_config_model,
};
use a4n_services::image::configured_image_provider;
use a4n_services::video::{
    UpdateVideoConfigParams as ServiceUpdateVideoConfigParams, VideoConfigParams,
    pick_provider_for_config, validate_video_config_model,
};
use reqwest::{Client, StatusCode};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::future::Future;
use std::time::Duration;

const LLM_CONNECTIVITY_TEST_PROMPT: &str = "ping";
const EMBEDDING_CONNECTIVITY_TEST_INPUT: &str = "test";
const IMAGE_CONNECTIVITY_TEST_PROMPT: &str = "test connectivity, draw a tiny dot";
const MODEL_DISCOVERY_CONNECT_TIMEOUT_SECS: u64 = 20;
const MODEL_DISCOVERY_READ_TIMEOUT_SECS: u64 = 60;
const MODEL_DISCOVERY_ERROR_PREVIEW_CHARS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub request_options_json: Option<Value>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub request_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateLlmInput {
    pub name: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string_update")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: Option<String>,
    pub max_concurrent: Option<i32>,
    pub timeout_secs: Option<i32>,
    pub max_context_tokens: Option<i32>,
    pub max_output_tokens: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_json_update")]
    pub request_options_json: Option<Option<Value>>,
}

fn deserialize_optional_string_update<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(deserializer)?))
}

fn deserialize_optional_json_update<'de, D>(
    deserializer: D,
) -> Result<Option<Option<Value>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(Option::<Value>::deserialize(deserializer)?))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub has_api_key: bool,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateEmbeddingInput {
    pub name: Option<String>,
    pub backend: Option<String>,
    pub base_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string_update")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: Option<String>,
    pub dimension: Option<i32>,
    pub batch_size: Option<i32>,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub has_api_key: bool,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub provider_options_json: Option<Value>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateImageConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(default, deserialize_with = "deserialize_optional_string_update")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub has_api_key: bool,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateVideoConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(default, deserialize_with = "deserialize_optional_string_update")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDiscoveryInput {
    pub provider: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    #[serde(default)]
    pub config_id: Option<i64>,
    #[serde(default)]
    pub use_saved_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredModel {
    pub id: String,
    pub name: Option<String>,
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDiscoveryResult {
    pub models: Vec<DiscoveredModel>,
}

#[derive(Debug, Clone, Serialize)]
struct EmbeddingConnectivityPreviewRequest {
    backend: String,
    model: String,
    dimension: i32,
    input: Vec<String>,
}

pub async fn list_llm_config(state: &AppState) -> Result<Vec<LlmConfigSummary>, String> {
    let configs = state
        .config_svc
        .list_llm()
        .await
        .map_err(|e| e.to_string())?;
    Ok(configs.iter().map(llm_config_summary).collect())
}

pub async fn create_llm_config(
    state: &AppState,
    input: CreateLlmInput,
) -> Result<LlmConfigSummary, String> {
    let config = state
        .config_svc
        .create_llm(a4n_services::config::CreateLlmConfigInput {
            name: &input.name,
            provider: &input.provider,
            base_url: &input.base_url,
            api_key_encrypted: input.api_key_encrypted.as_deref(),
            model: &input.model,
            max_concurrent: input.max_concurrent,
            timeout_secs: input.timeout_secs,
            max_context_tokens: input.max_context_tokens,
            max_output_tokens: input.max_output_tokens,
            request_options_json: input.request_options_json,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(llm_config_summary(&config))
}

pub async fn update_llm_config(
    state: &AppState,
    config_id: i64,
    input: UpdateLlmInput,
) -> Result<LlmConfigSummary, String> {
    let existing = state
        .config_svc
        .get_llm(config_id)
        .await
        .map_err(|e| e.to_string())?;
    if existing.is_default {
        let next_model_max_output = input
            .max_output_tokens
            .unwrap_or(existing.max_output_tokens);
        let model_cap = positive_model_output_cap(next_model_max_output)?;
        let max_tokens = load_persisted_llm_max_tokens_config(state).await?;
        max_tokens.validate_against_model_output_cap(model_cap)?;
    }
    let config = state
        .config_svc
        .update_llm(
            config_id,
            a4n_services::config::UpdateLlmConfigInput {
                name: input.name.as_deref(),
                provider: input.provider.as_deref(),
                base_url: input.base_url.as_deref(),
                api_key_encrypted: input
                    .api_key_encrypted
                    .as_ref()
                    .map(|value| value.as_deref()),
                model: input.model.as_deref(),
                max_concurrent: input.max_concurrent,
                timeout_secs: input.timeout_secs,
                max_context_tokens: input.max_context_tokens,
                max_output_tokens: input.max_output_tokens,
                request_options_json: input.request_options_json,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(llm_config_summary(&config))
}

pub async fn set_default_llm_config(state: &AppState, config_id: i64) -> Result<(), String> {
    let target = state
        .config_svc
        .get_llm(config_id)
        .await
        .map_err(|e| e.to_string())?;
    let model_cap = positive_model_output_cap(target.max_output_tokens)?;
    let max_tokens = load_persisted_llm_max_tokens_config(state).await?;
    max_tokens.validate_against_model_output_cap(model_cap)?;
    state
        .config_svc
        .set_default_llm(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_llm_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .config_svc
        .delete_llm(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn test_llm_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_llm_config_for_connectivity_test(state, config_id).await?;
    let timeout_secs = positive_i32_as_u64(config.timeout_secs, "LLM 配置 timeout_secs")?;
    let provider = a4n_engine::llm::http::HttpLlmProvider::new_with_timeout_and_options(
        &config.base_url,
        &config.model,
        config.api_key_encrypted.clone(),
        timeout_secs,
        config.request_options_json.clone(),
    )
    .map_err(|e| e.to_string())?;
    let request = build_llm_connectivity_test_request(state, &config).await?;
    let response = state
        .task_queue
        .run_tracked(
            QueueTaskType::LlmCall,
            TaskPriority::High,
            None,
            || async move { provider.chat(request).await },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(!response.message.content.is_empty())
}

pub async fn preview_test_llm_config_request(
    state: &AppState,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_llm_config_for_connectivity_test(state, config_id).await?;
    let request = build_llm_connectivity_test_request(state, &config).await?;
    let context_cap = positive_model_context_cap(config.max_context_tokens)?;
    let output_cap = positive_model_output_cap(config.max_output_tokens)?;
    Ok(model_request_preview(
        "llm_config_connectivity_test",
        "LLM 配置测试请求预览",
        vec![chat_request_step_with_budget(
            ChatRequestStepWithBudgetInput {
                step: ChatRequestStepInput {
                    key: "llm_connectivity_test".to_string(),
                    title: "LLM 连接测试请求".to_string(),
                    prompt_name: None,
                    state: "ready".to_string(),
                    note: Some("固定连接测试请求：ping".to_string()),
                    request,
                },
                model_context_tokens: context_cap,
                output_token_reserve_override: Some(output_cap),
                budget_note: Some(format!(
                    "被测配置模型={}；上下文={}；输出上限={}",
                    config.model, context_cap, output_cap
                )),
            },
        )?],
    ))
}

pub async fn detect_llm_config_models(
    state: &AppState,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    let api_key = model_discovery_api_key(input.api_key_encrypted.as_deref(), || async {
        if !input.use_saved_api_key {
            return Ok(None);
        }
        let config_id = required_existing_config_id(input.config_id, "LLM")?;
        let config = state
            .config_svc
            .get_llm(config_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(config.api_key_encrypted)
    })
    .await?;
    discover_models(&input.provider, &input.base_url, api_key.as_deref()).await
}

async fn resolve_llm_config_for_connectivity_test(
    state: &AppState,
    config_id: i64,
) -> Result<llm_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_llm_model() {
        validate_llm_runtime_snapshot_config_id(&config, config_id)?;
        return Ok(config);
    }
    state
        .config_svc
        .get_llm(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_llm_max_tokens_config(state: &AppState) -> Result<LlmMaxTokensConfig, String> {
    crate::dynamic_llm::load_llm_max_tokens_config(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())
}

pub async fn update_llm_max_tokens_config(
    state: &AppState,
    input: LlmMaxTokensConfig,
) -> Result<LlmMaxTokensConfig, String> {
    input.validate()?;
    for (label, model_cap) in persisted_llm_default_model_output_caps(state).await? {
        input
            .validate_against_model_output_cap(model_cap)
            .map_err(|err| format!("{label}：{err}"))?;
    }
    let value = serde_json::to_value(&input).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().naive_utc();
    let existing = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(crate::dynamic_llm::LLM_MAX_TOKENS_SETTING_KEY))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    if let Some(setting) = existing {
        let mut active: system_settings::ActiveModel = setting.into();
        active.value = Set(value);
        active.category = Set("llm".to_string());
        active.description = Set(Some("LLM 各场景请求输出预算".to_string()));
        active.updated_at = Set(now);
        active
            .update(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
    } else {
        system_settings::ActiveModel {
            key: Set(crate::dynamic_llm::LLM_MAX_TOKENS_SETTING_KEY.to_string()),
            value: Set(value),
            category: Set("llm".to_string()),
            description: Set(Some("LLM 各场景请求输出预算".to_string())),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(input)
}

async fn load_effective_llm_max_tokens_config(
    state: &AppState,
) -> Result<LlmMaxTokensConfig, String> {
    match crate::runtime_config::current_llm_max_tokens() {
        Some(config) => {
            config.validate()?;
            Ok(config)
        }
        None => crate::dynamic_llm::load_llm_max_tokens_config(state.db.as_ref())
            .await
            .map_err(|e| e.to_string()),
    }
}

async fn load_persisted_llm_max_tokens_config(
    state: &AppState,
) -> Result<LlmMaxTokensConfig, String> {
    crate::dynamic_llm::load_llm_max_tokens_config(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())
}

async fn persisted_llm_default_model_output_caps(
    state: &AppState,
) -> Result<Vec<(String, usize)>, String> {
    let mut caps = Vec::new();
    if let Some(model) = llm_config::Entity::find()
        .filter(llm_config::Column::IsDefault.eq(true))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        caps.push((
            format!("默认 LLM 配置 #{} {}", model.id, model.name),
            positive_model_output_cap(model.max_output_tokens)?,
        ));
    }
    Ok(caps)
}

fn positive_model_output_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型最大输出必须大于 0".to_string())
}

fn positive_model_context_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型上下文窗口必须大于 0".to_string())
}

pub async fn list_embedding_config(
    state: &AppState,
) -> Result<Vec<EmbeddingConfigSummary>, String> {
    let configs = state
        .config_svc
        .list_embedding()
        .await
        .map_err(|e| e.to_string())?;
    Ok(configs.iter().map(embedding_config_summary).collect())
}

pub async fn create_embedding_config(
    state: &AppState,
    input: CreateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, String> {
    let config = state
        .config_svc
        .create_embedding(CreateEmbeddingConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: input.api_key_encrypted,
            model: input.model,
            dimension: input.dimension,
            batch_size: input.batch_size,
            local_model_path: input.local_model_path,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(embedding_config_summary(&config))
}

pub async fn update_embedding_config(
    state: &AppState,
    config_id: i64,
    input: UpdateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, String> {
    let existing = state
        .config_svc
        .get_embedding(config_id)
        .await
        .map_err(|e| e.to_string())?;
    let updated = state
        .config_svc
        .update_embedding(
            config_id,
            UpdateEmbeddingConfigParams {
                name: input.name.unwrap_or(existing.name),
                backend: input.backend.unwrap_or(existing.backend),
                base_url: input.base_url.or(existing.base_url),
                api_key_encrypted: input.api_key_encrypted,
                model: input.model.unwrap_or(existing.model),
                dimension: input.dimension.unwrap_or(existing.dimension),
                batch_size: input.batch_size.unwrap_or(existing.batch_size),
                local_model_path: input.local_model_path.or(existing.local_model_path),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(embedding_config_summary(&updated))
}

pub async fn set_default_embedding_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .config_svc
        .set_default_embedding(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_embedding_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .config_svc
        .delete_embedding(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn test_embedding_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_embedding_config_for_connectivity_test(state, config_id).await?;
    let expected_dimension = positive_i32_as_usize(config.dimension, "嵌入配置维度")?;
    let provider = crate::dynamic_embedding::make_provider_from_model(
        &config,
        crate::dynamic_embedding::default_fastembed_cache_dir(&state.config),
    )
    .map_err(|e| e.to_string())?;
    let vectors = state
        .task_queue
        .run_tracked(
            QueueTaskType::EmbeddingCall,
            TaskPriority::High,
            None,
            || async move {
                provider
                    .embed(vec![EMBEDDING_CONNECTIVITY_TEST_INPUT.to_string()])
                    .await
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(vectors
        .first()
        .is_some_and(|vector| vector.len() == expected_dimension))
}

pub async fn preview_test_embedding_config_request(
    state: &AppState,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_embedding_config_for_connectivity_test(state, config_id).await?;
    let request = build_embedding_connectivity_test_request(&config);
    Ok(model_request_preview(
        "embedding_config_connectivity_test",
        "Embedding 配置测试请求预览",
        vec![provider_request_step(
            "embedding_connectivity_test",
            "Embedding 连接测试请求",
            "embedding",
            None,
            "ready",
            Some("固定连接测试输入：test".to_string()),
            &request,
        )?],
    ))
}

async fn resolve_embedding_config_for_connectivity_test(
    state: &AppState,
    config_id: i64,
) -> Result<embedding_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_embedding_model() {
        validate_runtime_snapshot_config_id("embedding", config.id, config_id)?;
        return Ok(config);
    }
    state
        .config_svc
        .get_embedding(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub fn list_embedding_backends() -> Vec<String> {
    vec!["api".into(), "fastembed".into(), "local_hash".into()]
}

pub async fn list_image_configs(state: &AppState) -> Result<Vec<ImageConfigSummary>, String> {
    let configs = state
        .config_svc
        .list_image()
        .await
        .map_err(|e| e.to_string())?;
    Ok(configs.iter().map(image_config_summary).collect())
}

pub async fn add_image_config(
    state: &AppState,
    input: ImageConfigInput,
) -> Result<ImageConfigSummary, String> {
    let config = state
        .config_svc
        .create_image(image_config_params(input))
        .await
        .map_err(|e| e.to_string())?;
    Ok(image_config_summary(&config))
}

pub async fn update_image_config(
    state: &AppState,
    config_id: i64,
    input: UpdateImageConfigInput,
) -> Result<ImageConfigSummary, String> {
    let updated = state
        .config_svc
        .update_image(config_id, image_update_config_params(input))
        .await
        .map_err(|e| e.to_string())?;
    Ok(image_config_summary(&updated))
}

pub async fn remove_image_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .config_svc
        .delete_image(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn activate_image_config(
    state: &AppState,
    config_id: i64,
) -> Result<ImageConfigSummary, String> {
    let target = state
        .config_svc
        .get_image(config_id)
        .await
        .map_err(|e| e.to_string())?;
    validate_image_config_model(&target)
        .map(drop)
        .map_err(|e| e.to_string())?;
    let updated = state
        .config_svc
        .set_default_image(config_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(image_config_summary(&updated))
}

pub async fn test_image_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_image_config_for_connectivity_test(state, config_id).await?;
    let provider = configured_image_provider(&config).map_err(|e| e.to_string())?;
    let request = build_image_connectivity_test_request(&config);
    state
        .task_queue
        .run_tracked(
            QueueTaskType::ImageGeneration,
            TaskPriority::High,
            None,
            || async move { provider.generate(request).await.map(|_| ()) },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

pub async fn preview_test_image_config_request(
    state: &AppState,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_image_config_for_connectivity_test(state, config_id).await?;
    let request = build_image_connectivity_test_request(&config);
    Ok(model_request_preview(
        "image_config_connectivity_test",
        "图片配置测试请求预览",
        vec![image_provider_request_step(
            "image_connectivity_test",
            "图片连接测试 provider 请求",
            None,
            "ready",
            Some("固定连接测试请求，不包含参考图".to_string()),
            &config.backend,
            request,
            config.provider_options_json.as_ref(),
        )?],
    ))
}

pub async fn detect_image_config_models(
    state: &AppState,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    let api_key = model_discovery_api_key(input.api_key_encrypted.as_deref(), || async {
        if !input.use_saved_api_key {
            return Ok(None);
        }
        let config_id = required_existing_config_id(input.config_id, "图片")?;
        let config = state
            .config_svc
            .get_image(config_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(config.api_key_encrypted)
    })
    .await?;
    discover_models(&input.provider, &input.base_url, api_key.as_deref()).await
}

async fn resolve_image_config_for_connectivity_test(
    state: &AppState,
    config_id: i64,
) -> Result<image_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_image_model() {
        validate_runtime_snapshot_config_id("image", config.id, config_id)?;
        return Ok(config);
    }
    state
        .config_svc
        .get_image(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_video_configs(state: &AppState) -> Result<Vec<VideoConfigSummary>, String> {
    let configs = state
        .config_svc
        .list_video()
        .await
        .map_err(|e| e.to_string())?;
    Ok(configs.iter().map(video_config_summary).collect())
}

pub async fn add_video_config(
    state: &AppState,
    input: VideoConfigInput,
) -> Result<VideoConfigSummary, String> {
    let config = state
        .config_svc
        .create_video(video_config_params(input))
        .await
        .map_err(|e| e.to_string())?;
    Ok(video_config_summary(&config))
}

pub async fn update_video_config(
    state: &AppState,
    config_id: i64,
    input: UpdateVideoConfigInput,
) -> Result<VideoConfigSummary, String> {
    let updated = state
        .config_svc
        .update_video(config_id, video_update_config_params(input))
        .await
        .map_err(|e| e.to_string())?;
    Ok(video_config_summary(&updated))
}

pub async fn remove_video_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .config_svc
        .delete_video(config_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn activate_video_config(
    state: &AppState,
    config_id: i64,
) -> Result<VideoConfigSummary, String> {
    let target = state
        .config_svc
        .get_video(config_id)
        .await
        .map_err(|e| e.to_string())?;
    validate_video_config_model(&target).map_err(|e| e.to_string())?;
    let updated = state
        .config_svc
        .set_default_video(config_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(video_config_summary(&updated))
}

pub async fn test_video_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_video_config_for_connectivity_test(state, config_id).await?;
    pick_provider_for_config(&config)?;
    Ok(true)
}

pub async fn detect_video_config_models(
    state: &AppState,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    let api_key = model_discovery_api_key(input.api_key_encrypted.as_deref(), || async {
        if !input.use_saved_api_key {
            return Ok(None);
        }
        let config_id = required_existing_config_id(input.config_id, "视频")?;
        let config = state
            .config_svc
            .get_video(config_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(config.api_key_encrypted)
    })
    .await?;
    discover_models(&input.provider, &input.base_url, api_key.as_deref()).await
}

async fn resolve_video_config_for_connectivity_test(
    state: &AppState,
    config_id: i64,
) -> Result<video_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_video_model() {
        validate_runtime_snapshot_config_id("video", config.id, config_id)?;
        return Ok(config);
    }
    state
        .config_svc
        .get_video(config_id)
        .await
        .map_err(|e| e.to_string())
}

async fn model_discovery_api_key<F, Fut>(
    provided_api_key: Option<&str>,
    load_saved_api_key: F,
) -> Result<Option<String>, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Option<String>, String>>,
{
    if let Some(api_key) = provided_api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(api_key.to_string()));
    }
    Ok(load_saved_api_key()
        .await?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn required_existing_config_id(config_id: Option<i64>, label: &str) -> Result<i64, String> {
    config_id
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label}模型探测需要有效的已保存配置 ID"))
}

#[derive(Debug, Clone, Copy)]
enum ModelDiscoveryAuth {
    Bearer,
    GeminiApiKey,
    None,
}

#[derive(Debug, Clone, Copy)]
enum ModelDiscoveryParser {
    OpenAi,
    Gemini,
    StableDiffusion,
    ComfyUiCheckpointLoader,
}

#[derive(Debug, Clone)]
struct ModelDiscoveryEndpoint {
    url: String,
    auth: ModelDiscoveryAuth,
    parser: ModelDiscoveryParser,
}

async fn discover_models(
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<ModelDiscoveryResult, String> {
    let base_url = required_model_discovery_base_url(base_url)?;
    let endpoints = model_discovery_endpoints(provider, &base_url);
    if endpoints.is_empty() {
        return Err(format!("{} 不支持自动探测模型列表", provider.trim()));
    }
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(MODEL_DISCOVERY_CONNECT_TIMEOUT_SECS))
        .read_timeout(Duration::from_secs(MODEL_DISCOVERY_READ_TIMEOUT_SECS))
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("模型探测 HTTP 客户端初始化失败：{e}"))?;

    let mut errors = Vec::new();
    for endpoint in endpoints {
        match fetch_discovered_models(&client, &endpoint, api_key).await {
            Ok(models) if !models.is_empty() => {
                return Ok(ModelDiscoveryResult { models });
            }
            Ok(_) => errors.push(format!("{}：响应中没有可识别模型", endpoint.url)),
            Err(err) => errors.push(err),
        }
    }
    Err(format!("模型探测失败：{}", errors.join("；")))
}

fn required_model_discovery_base_url(base_url: &str) -> Result<String, String> {
    let base_url = base_url.trim().trim_end_matches('/').to_string();
    if base_url.is_empty() {
        return Err("请先填写 API Base URL".to_string());
    }
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("API Base URL 必须以 http:// 或 https:// 开头".to_string());
    }
    Ok(base_url)
}

fn model_discovery_endpoints(provider: &str, base_url: &str) -> Vec<ModelDiscoveryEndpoint> {
    let provider = provider.trim().to_ascii_lowercase();
    if provider == "stable-diffusion" {
        return vec![ModelDiscoveryEndpoint {
            url: format!("{base_url}/sdapi/v1/sd-models"),
            auth: ModelDiscoveryAuth::None,
            parser: ModelDiscoveryParser::StableDiffusion,
        }];
    }
    if provider == "comfyui" {
        return vec![ModelDiscoveryEndpoint {
            url: format!("{base_url}/object_info/CheckpointLoaderSimple"),
            auth: ModelDiscoveryAuth::None,
            parser: ModelDiscoveryParser::ComfyUiCheckpointLoader,
        }];
    }
    if provider.contains("gemini") || base_url.contains("generativelanguage.googleapis.com") {
        return vec![ModelDiscoveryEndpoint {
            url: format!("{}/models", gemini_api_base_url(base_url)),
            auth: ModelDiscoveryAuth::GeminiApiKey,
            parser: ModelDiscoveryParser::Gemini,
        }];
    }

    openai_model_list_urls(base_url)
        .into_iter()
        .map(|url| ModelDiscoveryEndpoint {
            url,
            auth: ModelDiscoveryAuth::Bearer,
            parser: ModelDiscoveryParser::OpenAi,
        })
        .collect()
}

fn gemini_api_base_url(base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    if base_url.ends_with("/models") {
        return base_url
            .trim_end_matches("/models")
            .trim_end_matches('/')
            .to_string();
    }
    if base_url.ends_with("/v1beta") {
        base_url.to_string()
    } else if let Some(prefix) = base_url.strip_suffix("/v1") {
        format!("{prefix}/v1beta")
    } else {
        format!("{base_url}/v1beta")
    }
}

fn openai_model_list_urls(base_url: &str) -> Vec<String> {
    let base = strip_known_model_endpoint_suffix(base_url);
    let mut urls = Vec::new();
    let primary = if base.ends_with("/models") {
        base.clone()
    } else {
        format!("{base}/models")
    };
    push_unique_url(&mut urls, primary);
    if !base.ends_with("/v1")
        && !base.ends_with("/v1beta")
        && !base.ends_with("/models")
        && !base.contains("/v1/")
        && !base.contains("/v1beta/")
    {
        push_unique_url(&mut urls, format!("{base}/v1/models"));
    }
    urls
}

fn strip_known_model_endpoint_suffix(base_url: &str) -> String {
    let suffixes = [
        "/chat/completions",
        "/responses",
        "/images/generations",
        "/images/edits",
        "/videos/generations",
        "/video/generations",
        "/videos",
        "/task/submit",
    ];
    let mut base = base_url.trim_end_matches('/').to_string();
    loop {
        let lower = base.to_ascii_lowercase();
        let Some(suffix) = suffixes.iter().find(|suffix| lower.ends_with(**suffix)) else {
            return base;
        };
        let next_len = base.len() - suffix.len();
        base.truncate(next_len);
        base = base.trim_end_matches('/').to_string();
    }
}

fn push_unique_url(urls: &mut Vec<String>, url: String) {
    if !urls.iter().any(|existing| existing == &url) {
        urls.push(url);
    }
}

async fn fetch_discovered_models(
    client: &Client,
    endpoint: &ModelDiscoveryEndpoint,
    api_key: Option<&str>,
) -> Result<Vec<DiscoveredModel>, String> {
    let mut request = client
        .get(&endpoint.url)
        .header("accept", "application/json");
    match endpoint.auth {
        ModelDiscoveryAuth::Bearer => {
            if let Some(api_key) = api_key {
                request = request.bearer_auth(api_key);
            }
        }
        ModelDiscoveryAuth::GeminiApiKey => {
            if let Some(api_key) = api_key {
                request = request.header("x-goog-api-key", api_key);
            }
        }
        ModelDiscoveryAuth::None => {}
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("{} 请求失败：{e}", endpoint.url))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("{} 响应读取失败：{e}", endpoint.url))?;
    if !status.is_success() {
        return Err(model_discovery_status_error(&endpoint.url, status, &text));
    }
    let json: Value = serde_json::from_str(&text).map_err(|e| {
        format!(
            "{} 响应不是合法 JSON：{e}；响应片段：{}",
            endpoint.url,
            preview_model_discovery_body(&text)
        )
    })?;
    parse_discovered_models(&json, endpoint.parser).map_err(|e| {
        format!(
            "{} 响应结构无法识别：{e}；响应片段：{}",
            endpoint.url,
            preview_model_discovery_body(&text)
        )
    })
}

fn model_discovery_status_error(url: &str, status: StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| model_discovery_upstream_error(&value))
        .unwrap_or_else(|| preview_model_discovery_body(body));
    format!("{url} 返回 {status}：{detail}")
}

fn model_discovery_upstream_error(value: &Value) -> Option<String> {
    let error = value.get("error").unwrap_or(value);
    if let Some(message) = error
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(message.to_string());
    }
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    Some(match code {
        Some(code) => format!("{message}; code={code}"),
        None => message.to_string(),
    })
}

fn preview_model_discovery_body(body: &str) -> String {
    body.chars()
        .take(MODEL_DISCOVERY_ERROR_PREVIEW_CHARS)
        .collect()
}

fn parse_discovered_models(
    value: &Value,
    parser: ModelDiscoveryParser,
) -> Result<Vec<DiscoveredModel>, String> {
    let models = match parser {
        ModelDiscoveryParser::OpenAi => parse_openai_models(value)?,
        ModelDiscoveryParser::Gemini => parse_gemini_models(value)?,
        ModelDiscoveryParser::StableDiffusion => parse_stable_diffusion_models(value)?,
        ModelDiscoveryParser::ComfyUiCheckpointLoader => parse_comfyui_checkpoint_models(value)?,
    };
    Ok(dedupe_discovered_models(models))
}

fn parse_openai_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    if let Some(items) = value.get("data").and_then(Value::as_array) {
        return Ok(items.iter().filter_map(model_from_openai_item).collect());
    }
    if let Some(items) = value.get("models").and_then(Value::as_array) {
        return Ok(items.iter().filter_map(model_from_openai_item).collect());
    }
    if let Some(items) = value.as_array() {
        return Ok(items.iter().filter_map(model_from_openai_item).collect());
    }
    Err("缺少 data[] 或 models[]".to_string())
}

fn parse_gemini_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| "缺少 models[]".to_string())?;
    Ok(items.iter().filter_map(model_from_gemini_item).collect())
}

fn parse_stable_diffusion_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "SD-WebUI sd-models 响应必须是数组".to_string())?;
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = trimmed_json_string(item.get("model_name"))
                .or_else(|| trimmed_json_string(item.get("title")))?;
            Some(DiscoveredModel {
                id: id.to_string(),
                name: trimmed_json_string(item.get("title")).map(str::to_string),
                owned_by: None,
            })
        })
        .collect())
}

fn parse_comfyui_checkpoint_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let ckpt_name = value
        .pointer("/CheckpointLoaderSimple/input/required/ckpt_name/0")
        .or_else(|| value.pointer("/input/required/ckpt_name/0"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "ComfyUI 响应缺少 CheckpointLoaderSimple.input.required.ckpt_name".to_string()
        })?;
    Ok(ckpt_name
        .iter()
        .filter_map(|item| {
            trimmed_json_string(Some(item)).map(|id| DiscoveredModel {
                id: id.to_string(),
                name: None,
                owned_by: None,
            })
        })
        .collect())
}

fn model_from_openai_item(value: &Value) -> Option<DiscoveredModel> {
    match value {
        Value::String(id) => non_empty_trimmed(id).map(|id| DiscoveredModel {
            id: id.to_string(),
            name: None,
            owned_by: None,
        }),
        Value::Object(item) => {
            let id = trimmed_json_string(item.get("id"))
                .or_else(|| trimmed_json_string(item.get("name")))
                .or_else(|| trimmed_json_string(item.get("model")))?;
            Some(DiscoveredModel {
                id: strip_models_prefix(id).to_string(),
                name: trimmed_json_string(item.get("name"))
                    .or_else(|| trimmed_json_string(item.get("display_name")))
                    .map(str::to_string),
                owned_by: trimmed_json_string(item.get("owned_by"))
                    .or_else(|| trimmed_json_string(item.get("owner")))
                    .map(str::to_string),
            })
        }
        _ => None,
    }
}

fn model_from_gemini_item(value: &Value) -> Option<DiscoveredModel> {
    let item = value.as_object()?;
    let raw_name = trimmed_json_string(item.get("name"))?;
    let id = strip_models_prefix(raw_name).to_string();
    Some(DiscoveredModel {
        id,
        name: trimmed_json_string(item.get("displayName"))
            .or_else(|| trimmed_json_string(item.get("display_name")))
            .map(str::to_string),
        owned_by: Some("google".to_string()),
    })
}

fn trimmed_json_string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).and_then(non_empty_trimmed)
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn strip_models_prefix(value: &str) -> &str {
    value.trim().strip_prefix("models/").unwrap_or(value.trim())
}

fn dedupe_discovered_models(models: Vec<DiscoveredModel>) -> Vec<DiscoveredModel> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for model in models {
        if seen.insert(model.id.clone()) {
            deduped.push(model);
        }
    }
    deduped
}

async fn build_llm_connectivity_test_request(
    state: &AppState,
    config: &llm_config::Model,
) -> Result<ChatRequest, String> {
    let max_tokens = load_effective_llm_max_tokens_config(state)
        .await?
        .connection_test;
    validate_llm_connection_test_max_tokens(max_tokens, config)?;
    Ok(ChatRequest::new(
        vec![ChatMessage::user(LLM_CONNECTIVITY_TEST_PROMPT)],
        Some(0.0),
        Some(max_tokens),
        None,
    ))
}

fn validate_llm_runtime_snapshot_config_id(
    config: &llm_config::Model,
    config_id: i64,
) -> Result<(), String> {
    validate_runtime_snapshot_config_id("llm", config.id, config_id)
}

fn validate_runtime_snapshot_config_id(
    config_kind: &str,
    snapshot_id: i64,
    config_id: i64,
) -> Result<(), String> {
    if snapshot_id == config_id {
        return Ok(());
    }
    Err(format!(
        "runtime_config_snapshot.{config_kind}.id 必须等于 config_id：snapshot_id={snapshot_id}, config_id={config_id}",
    ))
}

fn validate_llm_connection_test_max_tokens(
    max_tokens: usize,
    config: &llm_config::Model,
) -> Result<(), String> {
    let model_cap = positive_model_output_cap(config.max_output_tokens)?;
    if max_tokens <= model_cap {
        return Ok(());
    }
    Err(format!(
        "LLM 连接测试 max_tokens 不能超过被测配置模型最大输出：config_id={}, model={}, connection_test={}, model_max_output_tokens={}",
        config.id, config.model, max_tokens, model_cap
    ))
}

fn build_embedding_connectivity_test_request(
    config: &embedding_config::Model,
) -> EmbeddingConnectivityPreviewRequest {
    EmbeddingConnectivityPreviewRequest {
        backend: config.backend.clone(),
        model: config.model.clone(),
        dimension: config.dimension,
        input: vec![EMBEDDING_CONNECTIVITY_TEST_INPUT.to_string()],
    }
}

fn build_image_connectivity_test_request(
    config: &image_gen_config::Model,
) -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: IMAGE_CONNECTIVITY_TEST_PROMPT.to_string(),
        width: config.default_width,
        height: config.default_height,
        model: config.model.clone(),
        reference_images: Vec::new(),
    }
}

fn llm_config_summary(config: &llm_config::Model) -> LlmConfigSummary {
    LlmConfigSummary {
        id: config.id,
        name: config.name.clone(),
        provider: config.provider.clone(),
        base_url: config.base_url.clone(),
        has_api_key: config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        timeout_secs: config.timeout_secs,
        max_context_tokens: config.max_context_tokens,
        max_output_tokens: config.max_output_tokens,
        request_options_json: config.request_options_json.clone(),
        is_default: config.is_default,
        created_at: Some(config.created_at.to_string()),
        updated_at: Some(config.updated_at.to_string()),
    }
}

fn embedding_config_summary(config: &embedding_config::Model) -> EmbeddingConfigSummary {
    EmbeddingConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config.base_url.clone(),
        api_key_encrypted: None,
        has_api_key: config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        model: config.model.clone(),
        dimension: config.dimension,
        batch_size: config.batch_size,
        local_model_path: config.local_model_path.clone(),
        is_default: config.is_default,
        created_at: Some(config.created_at.to_string()),
        updated_at: Some(config.updated_at.to_string()),
    }
}

fn image_config_summary(config: &image_gen_config::Model) -> ImageConfigSummary {
    ImageConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config.base_url.clone(),
        api_key_encrypted: None,
        has_api_key: config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        default_width: config.default_width,
        default_height: config.default_height,
        provider_options_json: config.provider_options_json.clone(),
        is_default: config.is_default,
        created_at: Some(config.created_at.to_string()),
        updated_at: Some(config.updated_at.to_string()),
    }
}

fn video_config_summary(config: &video_gen_config::Model) -> VideoConfigSummary {
    VideoConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config.base_url.clone(),
        api_key_encrypted: None,
        has_api_key: config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        default_width: config.default_width,
        default_height: config.default_height,
        default_duration_secs: config.default_duration_secs,
        min_duration_secs: config.min_duration_secs,
        max_duration_secs: config.max_duration_secs,
        duration_mode: config.duration_mode.clone(),
        supported_durations_json: config.supported_durations_json.clone(),
        default_fps: config.default_fps,
        provider_options_json: config.provider_options_json.clone(),
        is_default: config.is_default,
        created_at: Some(config.created_at.to_string()),
        updated_at: Some(config.updated_at.to_string()),
    }
}

fn image_config_params(input: ImageConfigInput) -> CreateImageConfigParams {
    CreateImageConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        default_width: input.default_width,
        default_height: input.default_height,
        provider_options_json: input.provider_options_json,
    }
}

fn image_update_config_params(input: UpdateImageConfigInput) -> ServiceUpdateImageConfigParams {
    ServiceUpdateImageConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        default_width: input.default_width,
        default_height: input.default_height,
        provider_options_json: input.provider_options_json,
    }
}

fn video_config_params(input: VideoConfigInput) -> VideoConfigParams {
    VideoConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        default_width: input.default_width,
        default_height: input.default_height,
        default_duration_secs: input.default_duration_secs,
        min_duration_secs: input.min_duration_secs,
        max_duration_secs: input.max_duration_secs,
        duration_mode: input.duration_mode,
        supported_durations_json: input.supported_durations_json,
        default_fps: input.default_fps,
        provider_options_json: input.provider_options_json,
    }
}

fn video_update_config_params(input: UpdateVideoConfigInput) -> ServiceUpdateVideoConfigParams {
    ServiceUpdateVideoConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        default_width: input.default_width,
        default_height: input.default_height,
        default_duration_secs: input.default_duration_secs,
        min_duration_secs: input.min_duration_secs,
        max_duration_secs: input.max_duration_secs,
        duration_mode: input.duration_mode,
        supported_durations_json: input.supported_durations_json,
        default_fps: input.default_fps,
        provider_options_json: input.provider_options_json,
    }
}

fn positive_i32_as_u64(value: i32, label: &str) -> Result<u64, String> {
    u64::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label} 必须为正整数"))
}

fn positive_i32_as_usize(value: i32, label: &str) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label} 必须为正整数"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_llm_input_requires_model_token_limits() {
        let err = serde_json::from_value::<CreateLlmInput>(serde_json::json!({
            "name": "missing context cap",
            "provider": "openai",
            "base_url": "https://api.example/v1",
            "api_key_encrypted": "sk-test",
            "model": "gpt-test",
            "max_concurrent": 1,
            "timeout_secs": 30,
            "max_output_tokens": 8192,
            "request_options_json": null
        }))
        .expect_err("max_context_tokens is required by the current create contract");
        assert!(err.to_string().contains("max_context_tokens"));

        let err = serde_json::from_value::<CreateLlmInput>(serde_json::json!({
            "name": "missing output cap",
            "provider": "openai",
            "base_url": "https://api.example/v1",
            "api_key_encrypted": "sk-test",
            "model": "gpt-test",
            "max_concurrent": 1,
            "timeout_secs": 30,
            "max_context_tokens": 128000,
            "request_options_json": null
        }))
        .expect_err("max_output_tokens is required by the current create contract");
        assert!(err.to_string().contains("max_output_tokens"));
    }

    #[test]
    fn update_llm_input_distinguishes_api_key_update_intents() {
        let missing_key = serde_json::from_value::<UpdateLlmInput>(serde_json::json!({
            "name": "keep existing key"
        }))
        .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);

        let clear_key = serde_json::from_value::<UpdateLlmInput>(serde_json::json!({
            "api_key_encrypted": null
        }))
        .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));

        let replace_key = serde_json::from_value::<UpdateLlmInput>(serde_json::json!({
            "api_key_encrypted": "sk-new"
        }))
        .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            replace_key.api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }

    #[test]
    fn llm_connectivity_test_requires_snapshot_to_match_config_id() {
        let config = test_llm_config_model(7, 4096);

        validate_llm_runtime_snapshot_config_id(&config, 7)
            .expect("matching snapshot config id should be accepted");
        let err = validate_llm_runtime_snapshot_config_id(&config, 8)
            .expect_err("mismatched snapshot config id must be rejected");

        assert!(err.contains("runtime_config_snapshot.llm.id 必须等于 config_id"));
        assert!(err.contains("snapshot_id=7"));
        assert!(err.contains("config_id=8"));
    }

    #[test]
    fn runtime_snapshot_config_id_error_names_non_llm_config_kind() {
        let err = validate_runtime_snapshot_config_id("image", 3, 4)
            .expect_err("mismatched non-LLM snapshot config id must be rejected");

        assert!(err.contains("runtime_config_snapshot.image.id 必须等于 config_id"));
        assert!(err.contains("snapshot_id=3"));
        assert!(err.contains("config_id=4"));
    }

    #[test]
    fn llm_connectivity_test_rejects_connection_tokens_above_tested_model_cap() {
        let config = test_llm_config_model(7, 4);

        let err = validate_llm_connection_test_max_tokens(8, &config)
            .expect_err("connection_test above tested model output cap must fail");

        assert!(err.contains("LLM 连接测试 max_tokens 不能超过被测配置模型最大输出"));
        assert!(err.contains("config_id=7"));
        assert!(err.contains("connection_test=8"));
        assert!(err.contains("model_max_output_tokens=4"));
    }

    fn test_llm_config_model(id: i64, max_output_tokens: i32) -> llm_config::Model {
        let now = chrono::Utc::now().naive_utc();
        llm_config::Model {
            id,
            name: format!("test llm {id}"),
            provider: "openai".to_string(),
            base_url: "https://api.example/v1".to_string(),
            api_key_encrypted: Some("sk-test".to_string()),
            model: "gpt-test".to_string(),
            max_concurrent: 1,
            timeout_secs: 30,
            max_context_tokens: 128_000,
            max_output_tokens,
            request_options_json: None,
            is_default: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn create_embedding_input_requires_batch_size() {
        let err = serde_json::from_value::<CreateEmbeddingInput>(serde_json::json!({
            "name": "missing batch",
            "backend": "api",
            "base_url": "https://api.openai.com/v1",
            "model": "text-embedding-3-small",
            "dimension": 1536
        }))
        .expect_err("batch_size is required by the current create contract");

        assert!(err.to_string().contains("batch_size"));
    }

    #[test]
    fn update_embedding_input_distinguishes_api_key_update_intents() {
        let missing_key = serde_json::from_value::<UpdateEmbeddingInput>(serde_json::json!({
            "name": "keep existing embedding key"
        }))
        .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);

        let clear_key = serde_json::from_value::<UpdateEmbeddingInput>(serde_json::json!({
            "api_key_encrypted": null
        }))
        .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));

        let replace_key = serde_json::from_value::<UpdateEmbeddingInput>(serde_json::json!({
            "api_key_encrypted": "sk-new"
        }))
        .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            replace_key.api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }

    #[test]
    fn update_image_input_distinguishes_api_key_update_intents() {
        let missing_key = serde_json::from_value::<UpdateImageConfigInput>(serde_json::json!({
            "name": "keep existing image key",
            "backend": "openai-compatible-image",
            "base_url": "https://api.example/v1",
            "model": "image-model",
            "max_concurrent": 1,
            "default_width": 1024,
            "default_height": 1024,
            "provider_options_json": null
        }))
        .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);
        assert_eq!(
            image_update_config_params(missing_key).api_key_encrypted,
            None
        );

        let clear_key = serde_json::from_value::<UpdateImageConfigInput>(serde_json::json!({
            "name": "null image key",
            "backend": "openai-compatible-image",
            "base_url": "https://api.example/v1",
            "api_key_encrypted": null,
            "model": "image-model",
            "max_concurrent": 1,
            "default_width": 1024,
            "default_height": 1024,
            "provider_options_json": null
        }))
        .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));
        assert_eq!(
            image_update_config_params(clear_key).api_key_encrypted,
            Some(None)
        );

        let replace_key = serde_json::from_value::<UpdateImageConfigInput>(serde_json::json!({
            "name": "replace image key",
            "backend": "openai-compatible-image",
            "base_url": "https://api.example/v1",
            "api_key_encrypted": "sk-new",
            "model": "image-model",
            "max_concurrent": 1,
            "default_width": 1024,
            "default_height": 1024,
            "provider_options_json": null
        }))
        .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            image_update_config_params(replace_key).api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }

    #[test]
    fn update_video_input_distinguishes_api_key_update_intents() {
        let missing_key = serde_json::from_value::<UpdateVideoConfigInput>(serde_json::json!({
            "name": "keep existing video key",
            "backend": "openai-compatible-video",
            "base_url": "https://api.example/v1",
            "model": "video-model",
            "max_concurrent": 1,
            "default_width": 1280,
            "default_height": 720,
            "default_duration_secs": 6,
            "min_duration_secs": 1,
            "max_duration_secs": 60,
            "duration_mode": "range",
            "supported_durations_json": null,
            "default_fps": 24,
            "provider_options_json": null
        }))
        .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);
        assert_eq!(
            video_update_config_params(missing_key).api_key_encrypted,
            None
        );

        let clear_key = serde_json::from_value::<UpdateVideoConfigInput>(serde_json::json!({
            "name": "null video key",
            "backend": "openai-compatible-video",
            "base_url": "https://api.example/v1",
            "api_key_encrypted": null,
            "model": "video-model",
            "max_concurrent": 1,
            "default_width": 1280,
            "default_height": 720,
            "default_duration_secs": 6,
            "min_duration_secs": 1,
            "max_duration_secs": 60,
            "duration_mode": "range",
            "supported_durations_json": null,
            "default_fps": 24,
            "provider_options_json": null
        }))
        .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));
        assert_eq!(
            video_update_config_params(clear_key).api_key_encrypted,
            Some(None)
        );

        let replace_key = serde_json::from_value::<UpdateVideoConfigInput>(serde_json::json!({
            "name": "replace video key",
            "backend": "openai-compatible-video",
            "base_url": "https://api.example/v1",
            "api_key_encrypted": "sk-new",
            "model": "video-model",
            "max_concurrent": 1,
            "default_width": 1280,
            "default_height": 720,
            "default_duration_secs": 6,
            "min_duration_secs": 1,
            "max_duration_secs": 60,
            "duration_mode": "range",
            "supported_durations_json": null,
            "default_fps": 24,
            "provider_options_json": null
        }))
        .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            video_update_config_params(replace_key).api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }

    #[test]
    fn list_embedding_backends_is_stable_contract() {
        assert_eq!(
            list_embedding_backends(),
            vec!["api", "fastembed", "local_hash"]
        );
    }
}
