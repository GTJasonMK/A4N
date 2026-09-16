use a4n_domain::{ImageSize, ThemeDocument, ThemeMode};
use sea_orm::DbErr;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub(crate) fn is_retired_system_setting_key(key: &str) -> bool {
    key == "web_search"
}

#[derive(Clone, Debug)]
pub struct SettingsAllConfigImportInput {
    pub overwrite: bool,
    pub rows: SettingsAllConfigRows,
}

#[derive(Clone, Debug)]
pub struct SettingsAllConfigRows {
    pub llm_configs: Vec<SettingsAllLlmConfigRow>,
    pub embedding_configs: Vec<SettingsAllEmbeddingConfigRow>,
    pub image_configs: Vec<SettingsAllImageConfigRow>,
    pub video_configs: Vec<SettingsAllVideoConfigRow>,
    pub themes: Vec<SettingsAllThemeRow>,
    pub system_settings: Vec<SettingsAllSystemSettingRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAllLlmConfigRow {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub extra_body_json: Option<Value>,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAllEmbeddingConfigRow {
    pub name: String,
    pub backend: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub base_url: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub local_model_path: Option<String>,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAllImageConfigRow {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub supported_sizes: Vec<ImageSize>,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAllVideoConfigRow {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAllThemeRow {
    #[serde(deserialize_with = "deserialize_required_option")]
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: ThemeMode,
    pub document: ThemeDocument,
    pub is_system_default: bool,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAllSystemSettingRow {
    pub key: String,
    pub value: Value,
    pub category: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsAllImportTableCounts {
    pub imported: usize,
    pub skipped: usize,
}

impl SettingsAllImportTableCounts {
    pub(super) fn record(&mut self, imported: bool) {
        if imported {
            self.imported += 1;
        } else {
            self.skipped += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsAllConfigImportCounts {
    pub llm: SettingsAllImportTableCounts,
    pub embedding: SettingsAllImportTableCounts,
    pub image: SettingsAllImportTableCounts,
    pub video: SettingsAllImportTableCounts,
    pub theme: SettingsAllImportTableCounts,
    pub system_settings: SettingsAllImportTableCounts,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsAllConfigImportError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

pub(super) fn settings_all_config_database(
    operation: &'static str,
    source: DbErr,
) -> SettingsAllConfigImportError {
    SettingsAllConfigImportError::Database { operation, source }
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
