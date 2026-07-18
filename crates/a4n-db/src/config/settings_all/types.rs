use sea_orm::DbErr;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct SettingsAllConfigImportInput {
    pub overwrite: bool,
    pub llm_configs: Vec<SettingsAllLlmConfigImportRow>,
    pub embedding_configs: Vec<SettingsAllEmbeddingConfigImportRow>,
    pub image_configs: Vec<SettingsAllImageConfigImportRow>,
    pub video_configs: Vec<SettingsAllVideoConfigImportRow>,
    pub themes: Vec<SettingsAllThemeImportRow>,
    pub system_settings: Vec<SettingsAllSystemSettingImportRow>,
}

#[derive(Clone, Debug)]
pub struct SettingsAllLlmConfigImportRow {
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
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct SettingsAllEmbeddingConfigImportRow {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct SettingsAllImageConfigImportRow {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub provider_options_json: Option<Value>,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct SettingsAllVideoConfigImportRow {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
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
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct SettingsAllThemeImportRow {
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: String,
    pub css_variables: Value,
    pub is_system_default: bool,
    pub is_active: bool,
}

#[derive(Clone, Debug)]
pub struct SettingsAllSystemSettingImportRow {
    pub key: String,
    pub value: Value,
    pub category: String,
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
