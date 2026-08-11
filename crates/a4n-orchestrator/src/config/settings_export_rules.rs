//! 全量配置导出/导入校验纯规则（无 DB、无 Host）。

use std::collections::HashSet;

use a4n_core::{
    config::{
        CreateEmbeddingConfigParams, CreateImageConfigParams, app_config::LlmMaxTokensConfig,
    },
    error::AppError,
};
use a4n_llm::provider::normalize_extra_body_json;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    config::{
        LLM_MAX_TOKENS_SETTING_KEY, SettingsAllConfigImportCounts, SettingsAllEmbeddingConfigRow,
        SettingsAllImageConfigRow, SettingsAllImportTableCounts, SettingsAllLlmConfigRow,
        SettingsAllSystemSettingRow, SettingsAllThemeRow, SettingsAllVideoConfigRow,
    },
    video::{VideoConfigParams, validate_video_config_params},
};

pub const SETTINGS_ALL_FORMAT_VERSION: &str = "3.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllConfigsExport {
    pub version: String,
    pub export_time: String,
    pub llm_configs: Vec<SettingsAllLlmConfigRow>,
    pub embedding_configs: Vec<SettingsAllEmbeddingConfigRow>,
    pub image_configs: Vec<SettingsAllImageConfigRow>,
    pub video_configs: Vec<SettingsAllVideoConfigRow>,
    pub prompts_modified: Vec<PromptRow>,
    pub themes: Vec<SettingsAllThemeRow>,
    pub system_settings: Vec<SettingsAllSystemSettingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptRow {
    pub name: String,
    pub content: String,
}

pub fn validate_all_configs_export_version(version: &str) -> Result<(), String> {
    if version != SETTINGS_ALL_FORMAT_VERSION {
        return Err(format!(
            "version 不匹配：期望 '{SETTINGS_ALL_FORMAT_VERSION}'，收到 '{version}'"
        ));
    }
    Ok(())
}

/// 最多一个默认项。
pub fn ensure_at_most_one_default(kind: &str, count: usize) -> Result<(), String> {
    if count > 1 {
        return Err(format!("{kind}最多只能有一个默认项，当前为 {count} 个"));
    }
    Ok(())
}

/// 唯一身份（name/key 等）。
pub fn ensure_unique_identity(
    seen: &mut HashSet<String>,
    kind: &str,
    value: impl AsRef<str>,
) -> Result<(), String> {
    let value = value.as_ref();
    if !seen.insert(value.to_string()) {
        return Err(format!("{kind} 重复：{value}"));
    }
    Ok(())
}

/// 必填非空文本（trim）。
pub fn required_text<'a>(value: &'a str, label: &str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} 缺少 {field}"));
    }
    Ok(value)
}

/// 正整数字段。
pub fn positive_i32(value: i32, label: &str, field: &str) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("{label} {field} 必须为正整数"));
    }
    Ok(())
}

/// 原地 trim String。
pub fn trim_string(value: &mut String) {
    let trimmed = value.trim();
    if trimmed.len() != value.len() {
        *value = trimmed.to_string();
    }
}

/// 默认命名行通用校验骨架。
pub fn validate_default_named_rows<T>(
    kind: &str,
    rows: &[T],
    is_default: impl Fn(&T) -> bool,
    name: impl for<'a> Fn(&'a T) -> &'a str,
) -> Result<(), String> {
    ensure_at_most_one_default(kind, rows.iter().filter(|row| is_default(*row)).count())?;

    let mut seen_names = HashSet::new();
    let identity_kind = format!("{kind} name");
    for (index, row) in rows.iter().enumerate() {
        let label = format!("{kind} #{}", index + 1);
        let name = required_text(name(row), &label, "name")?;
        ensure_unique_identity(&mut seen_names, &identity_kind, name)?;
    }
    Ok(())
}

pub(super) fn validate_all_configs_export(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_all_configs_export_version(&incoming.version)?;
    validate_provider_rows(incoming)?;
    validate_setting_rows(incoming)
}

pub(super) fn normalize_all_configs_export(incoming: &mut AllConfigsExport) -> Result<(), String> {
    normalize_provider_rows(incoming)?;
    normalize_setting_rows(incoming);
    Ok(())
}

pub(super) fn import_details_from_counts(
    counts: SettingsAllConfigImportCounts,
    prompt_imported: usize,
) -> Vec<String> {
    vec![
        import_detail("LLM 配置", counts.llm),
        import_detail("嵌入配置", counts.embedding),
        import_detail("图片配置", counts.image),
        import_detail("视频配置", counts.video),
        format!("提示词：导入 {prompt_imported}"),
        import_detail("主题", counts.theme),
        import_detail("系统设置", counts.system_settings),
    ]
}

fn validate_provider_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_llm_rows(incoming)?;
    validate_default_named_rows(
        "嵌入配置",
        &incoming.embedding_configs,
        |row| row.is_default,
        |row| &row.name,
    )?;
    validate_default_named_rows(
        "图片配置",
        &incoming.image_configs,
        |row| row.is_default,
        |row| &row.name,
    )?;
    validate_default_named_rows(
        "视频配置",
        &incoming.video_configs,
        |row| row.is_default,
        |row| &row.name,
    )
}

fn validate_llm_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "LLM 配置",
        &incoming.llm_configs,
        |row| row.is_default,
        |row| &row.name,
    )?;
    for (index, row) in incoming.llm_configs.iter().enumerate() {
        let label = format!("LLM 配置 #{}", index + 1);
        required_text(&row.provider, &label, "provider")?;
        required_text(&row.base_url, &label, "base_url")?;
        required_text(&row.model, &label, "model")?;
        positive_i32(row.max_concurrent, &label, "max_concurrent")?;
        positive_i32(row.timeout_secs, &label, "timeout_secs")?;
        positive_i32(row.max_context_tokens, &label, "max_context_tokens")?;
        positive_i32(row.max_output_tokens, &label, "max_output_tokens")?;
        if row.max_output_tokens >= row.max_context_tokens {
            return Err(format!(
                "{label} max_output_tokens 必须小于 max_context_tokens"
            ));
        }
    }
    Ok(())
}

fn normalize_provider_rows(incoming: &mut AllConfigsExport) -> Result<(), String> {
    for (index, row) in incoming.llm_configs.iter_mut().enumerate() {
        let label = format!("LLM 配置 #{}", index + 1);
        trim_string(&mut row.name);
        trim_string(&mut row.provider);
        trim_string(&mut row.base_url);
        trim_string(&mut row.model);
        row.extra_body_json =
            normalize_extra_body_json(row.extra_body_json.clone()).map_err(|error| {
                format!(
                    "{label} extra_body_json 规范化失败：{}",
                    plain_app_error(error)
                )
            })?;
    }
    for (index, row) in incoming.embedding_configs.iter_mut().enumerate() {
        let params =
            a4n_core::config::validate_embedding_config_params(embedding_params_from_row(row))
                .map_err(|error| format!("嵌入配置 #{} 规范化失败：{error}", index + 1))?;
        apply_embedding_params_to_row(row, params);
    }
    for (index, row) in incoming.image_configs.iter_mut().enumerate() {
        let params = a4n_core::config::validate_image_config_params(image_params_from_row(row))
            .map_err(|error| format!("图片配置 #{} 规范化失败：{error}", index + 1))?;
        apply_image_params_to_row(row, params);
    }
    for (index, row) in incoming.video_configs.iter_mut().enumerate() {
        let params = validate_video_config_params(video_params_from_row(row))
            .map_err(|error| format!("视频配置 #{} 规范化失败：{error}", index + 1))?;
        apply_video_params_to_row(row, params);
    }
    Ok(())
}

fn validate_setting_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    let mut prompt_names = HashSet::new();
    for (index, row) in incoming.prompts_modified.iter().enumerate() {
        let label = format!("提示词 #{}", index + 1);
        let name = required_text(&row.name, &label, "name")?;
        ensure_unique_identity(&mut prompt_names, "提示词 name", name)?;
        required_text(&row.content, &label, "content")?;
    }

    ensure_at_most_one_default(
        "系统默认主题",
        incoming
            .themes
            .iter()
            .filter(|row| row.is_system_default)
            .count(),
    )?;
    let mut theme_keys = HashSet::new();
    let mut active_scopes = HashSet::new();
    for (index, row) in incoming.themes.iter().enumerate() {
        let label = format!("主题 #{}", index + 1);
        let name = required_text(&row.name, &label, "name")?;
        if !theme_keys.insert((row.user_id, name.to_string())) {
            return Err(format!("主题 user_id/name 重复：{:?}/{name}", row.user_id));
        }
        if row.is_active && !active_scopes.insert(row.user_id) {
            return Err(format!("主题 active user_id 重复：{:?}", row.user_id));
        }
    }

    let mut setting_keys = HashSet::new();
    for (index, row) in incoming.system_settings.iter().enumerate() {
        let label = format!("系统设置 #{}", index + 1);
        let key = required_text(&row.key, &label, "key")?;
        ensure_unique_identity(&mut setting_keys, "系统设置 key", key)?;
        required_text(&row.category, &label, "category")?;
        if key == LLM_MAX_TOKENS_SETTING_KEY {
            validate_llm_max_tokens_setting(incoming, &label, row.value.clone())?;
        }
    }
    Ok(())
}

fn normalize_setting_rows(incoming: &mut AllConfigsExport) {
    for row in &mut incoming.prompts_modified {
        trim_string(&mut row.name);
    }
    for row in &mut incoming.themes {
        trim_string(&mut row.name);
    }
    for row in &mut incoming.system_settings {
        trim_string(&mut row.key);
        trim_string(&mut row.category);
    }
}

fn validate_llm_max_tokens_setting(
    incoming: &AllConfigsExport,
    label: &str,
    value: Value,
) -> Result<(), String> {
    let config: LlmMaxTokensConfig = serde_json::from_value(value)
        .map_err(|error| format!("{label} llm_max_tokens 解析失败：{error}"))?;
    config
        .validate()
        .map_err(|error| format!("{label} llm_max_tokens 无效：{error}"))?;
    if let Some(model_cap) = incoming
        .llm_configs
        .iter()
        .find(|row| row.is_default)
        .map(|row| usize::try_from(row.max_output_tokens))
        .transpose()
        .map_err(|_| format!("{label} 默认 LLM max_output_tokens 必须大于 0"))?
    {
        config
            .validate_against_model_output_cap(model_cap)
            .map_err(|error| format!("{label} llm_max_tokens 无效：{error}"))?;
    }
    Ok(())
}

fn embedding_params_from_row(row: &SettingsAllEmbeddingConfigRow) -> CreateEmbeddingConfigParams {
    CreateEmbeddingConfigParams {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        dimension: row.dimension,
        batch_size: row.batch_size,
        local_model_path: row.local_model_path.clone(),
    }
}

fn apply_embedding_params_to_row(
    row: &mut SettingsAllEmbeddingConfigRow,
    params: CreateEmbeddingConfigParams,
) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = params.base_url;
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.dimension = params.dimension;
    row.batch_size = params.batch_size;
    row.local_model_path = params.local_model_path;
}

fn image_params_from_row(row: &SettingsAllImageConfigRow) -> CreateImageConfigParams {
    CreateImageConfigParams {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        max_concurrent: row.max_concurrent,
        default_width: row.default_width,
        default_height: row.default_height,
    }
}

fn apply_image_params_to_row(row: &mut SettingsAllImageConfigRow, params: CreateImageConfigParams) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = params.base_url;
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.max_concurrent = params.max_concurrent;
    row.default_width = params.default_width;
    row.default_height = params.default_height;
}

fn video_params_from_row(row: &SettingsAllVideoConfigRow) -> VideoConfigParams {
    VideoConfigParams {
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
    }
}

fn apply_video_params_to_row(row: &mut SettingsAllVideoConfigRow, params: VideoConfigParams) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = params.base_url;
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.max_concurrent = params.max_concurrent;
    row.default_width = params.default_width;
    row.default_height = params.default_height;
    row.default_duration_secs = params.default_duration_secs;
    row.min_duration_secs = params.min_duration_secs;
    row.max_duration_secs = params.max_duration_secs;
    row.duration_mode = params.duration_mode;
    row.supported_durations_json = params.supported_durations_json;
    row.default_fps = params.default_fps;
}

fn import_detail(label: &str, counts: SettingsAllImportTableCounts) -> String {
    format!(
        "{}：导入 {}，跳过 {}",
        label, counts.imported, counts.skipped
    )
}

fn plain_app_error(error: AppError) -> String {
    match error {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn full_export() -> AllConfigsExport {
        serde_json::from_value(json!({
            "version": "3.0",
            "export_time": "2026-07-23T00:00:00Z",
            "llm_configs": [],
            "embedding_configs": [],
            "image_configs": [],
            "video_configs": [],
            "prompts_modified": [],
            "themes": [],
            "system_settings": []
        }))
        .unwrap()
    }

    #[test]
    fn header_and_shared_helpers() {
        validate_all_configs_export_version("3.0").unwrap();
        assert!(validate_all_configs_export_version("2.0").is_err());

        ensure_at_most_one_default("主题", 1).unwrap();
        assert!(ensure_at_most_one_default("主题", 2).is_err());

        let mut seen = HashSet::new();
        ensure_unique_identity(&mut seen, "name", "a").unwrap();
        assert!(ensure_unique_identity(&mut seen, "name", "a").is_err());

        assert_eq!(required_text("  x  ", "L", "f").unwrap(), "x");
        assert!(required_text("  ", "L", "f").is_err());
        positive_i32(3, "L", "n").unwrap();
        assert!(positive_i32(0, "L", "n").is_err());

        let mut s = "  ab  ".to_string();
        trim_string(&mut s);
        assert_eq!(s, "ab");
    }

    #[test]
    fn full_validation_rejects_trimmed_duplicate_prompt_names() {
        let mut export = full_export();
        export.prompts_modified = vec![
            PromptRow {
                name: " prompt ".to_string(),
                content: "first".to_string(),
            },
            PromptRow {
                name: "prompt".to_string(),
                content: "second".to_string(),
            },
        ];

        let error = validate_all_configs_export(&export).unwrap_err();

        assert_eq!(error, "提示词 name 重复：prompt");
    }

    #[test]
    fn normalization_reuses_provider_validators() {
        let mut export = full_export();
        export.llm_configs.push(SettingsAllLlmConfigRow {
            name: " main ".to_string(),
            provider: " openai ".to_string(),
            base_url: " https://example.com/v1 ".to_string(),
            api_key_encrypted: None,
            model: " model ".to_string(),
            max_concurrent: 1,
            timeout_secs: 30,
            max_context_tokens: 8192,
            max_output_tokens: 2048,
            extra_body_json: Some(json!({})),
            is_default: true,
        });

        validate_all_configs_export(&export).unwrap();
        normalize_all_configs_export(&mut export).unwrap();

        assert_eq!(export.llm_configs[0].name, "main");
        assert_eq!(export.llm_configs[0].provider, "openai");
        assert_eq!(export.llm_configs[0].model, "model");
    }

    #[test]
    fn encrypted_key_field_must_be_present_even_when_null() {
        let value = json!({
            "name": "main",
            "provider": "openai",
            "base_url": "https://example.com/v1",
            "model": "model",
            "max_concurrent": 1,
            "timeout_secs": 30,
            "max_context_tokens": 8192,
            "max_output_tokens": 2048,
            "extra_body_json": null,
            "is_default": true
        });

        assert!(serde_json::from_value::<SettingsAllLlmConfigRow>(value).is_err());
    }
}
