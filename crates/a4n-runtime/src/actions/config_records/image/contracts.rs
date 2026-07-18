use super::super::shared::deserialize_optional_string_update;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) use super::mapping::{
    image_config_summary, image_create_record_input, image_update_record_input,
};

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
