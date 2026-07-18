use super::super::shared::{deserialize_optional_json_update, deserialize_optional_string_update};
use a4n_core::models::ai::llm_config;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

pub(super) fn llm_config_summary(config: &llm_config::Model) -> LlmConfigSummary {
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

pub(super) fn positive_model_output_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型最大输出必须大于 0".to_string())
}

pub(super) fn positive_model_context_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型上下文窗口必须大于 0".to_string())
}
