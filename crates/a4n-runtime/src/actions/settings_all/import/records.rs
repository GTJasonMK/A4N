use super::super::types::{
    AllConfigsExport, EmbeddingRow, ImageRow, LlmRow, SystemSettingRow, ThemeRow, VideoRow,
};
use a4n_orchestrator::config::{
    SettingsAllConfigImportInput, SettingsAllEmbeddingConfigImportRow,
    SettingsAllImageConfigImportRow, SettingsAllLlmConfigImportRow,
    SettingsAllSystemSettingImportRow, SettingsAllThemeImportRow, SettingsAllVideoConfigImportRow,
};

pub(super) fn settings_all_config_import_input(
    incoming: &AllConfigsExport,
    overwrite: bool,
) -> SettingsAllConfigImportInput {
    SettingsAllConfigImportInput {
        overwrite,
        llm_configs: incoming.llm_configs.iter().map(llm_import_row).collect(),
        embedding_configs: incoming
            .embedding_configs
            .iter()
            .map(embedding_import_row)
            .collect(),
        image_configs: incoming
            .image_configs
            .iter()
            .map(image_import_row)
            .collect(),
        video_configs: incoming
            .video_configs
            .iter()
            .map(video_import_row)
            .collect(),
        themes: incoming.themes.iter().map(theme_import_row).collect(),
        system_settings: incoming
            .system_settings
            .iter()
            .map(system_setting_import_row)
            .collect(),
    }
}

fn llm_import_row(row: &LlmRow) -> SettingsAllLlmConfigImportRow {
    SettingsAllLlmConfigImportRow {
        name: row.name.clone(),
        provider: row.provider.clone(),
        base_url: row.base_url.clone(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        max_concurrent: row.max_concurrent,
        timeout_secs: row.timeout_secs,
        max_context_tokens: row.max_context_tokens,
        max_output_tokens: row.max_output_tokens,
        request_options_json: row.request_options_json.clone(),
        is_default: row.is_default,
    }
}

fn embedding_import_row(row: &EmbeddingRow) -> SettingsAllEmbeddingConfigImportRow {
    SettingsAllEmbeddingConfigImportRow {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        dimension: row.dimension,
        batch_size: row.batch_size,
        local_model_path: row.local_model_path.clone(),
        is_default: row.is_default,
    }
}

fn image_import_row(row: &ImageRow) -> SettingsAllImageConfigImportRow {
    SettingsAllImageConfigImportRow {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        max_concurrent: row.max_concurrent,
        default_width: row.default_width,
        default_height: row.default_height,
        provider_options_json: row.provider_options_json.clone(),
        is_default: row.is_default,
    }
}

fn video_import_row(row: &VideoRow) -> SettingsAllVideoConfigImportRow {
    SettingsAllVideoConfigImportRow {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        max_concurrent: row.max_concurrent,
        default_width: row.default_width,
        default_height: row.default_height,
        default_duration_secs: row.default_duration_secs,
        min_duration_secs: row.min_duration_secs,
        max_duration_secs: row.max_duration_secs,
        duration_mode: row.duration_mode.clone(),
        supported_durations_json: row.supported_durations_json.clone(),
        default_fps: row.default_fps,
        provider_options_json: row.provider_options_json.clone(),
        is_default: row.is_default,
    }
}

fn theme_import_row(row: &ThemeRow) -> SettingsAllThemeImportRow {
    SettingsAllThemeImportRow {
        user_id: row.user_id,
        name: row.name.clone(),
        mode: row.mode.clone(),
        css_variables: row.css_variables.clone(),
        is_system_default: row.is_system_default,
        is_active: row.is_active,
    }
}

fn system_setting_import_row(row: &SystemSettingRow) -> SettingsAllSystemSettingImportRow {
    SettingsAllSystemSettingImportRow {
        key: row.key.clone(),
        value: row.value.clone(),
        category: row.category.clone(),
        description: row.description.clone(),
    }
}
