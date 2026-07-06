use super::types::{AllConfigsExport, EmbeddingRow, ImageRow, LlmRow, VideoRow};
use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_core::error::AppError;
use a4n_engine::llm::http::normalize_request_options_json;
use a4n_services::config::{
    CreateEmbeddingConfigParams, CreateImageConfigParams, validate_embedding_config_params,
    validate_image_config_params,
};
use a4n_services::video::{VideoConfigParams, validate_video_config_params};
use std::collections::HashSet;

pub(super) fn validate_all_configs_export_header(
    incoming: &AllConfigsExport,
) -> Result<(), String> {
    if incoming.version != "1.0" {
        return Err(format!(
            "version 不匹配：期望 '1.0'，收到 '{}'",
            incoming.version
        ));
    }
    if incoming.export_type != "all" {
        return Err(format!(
            "export_type 不匹配：期望 'all'，收到 '{}'",
            incoming.export_type
        ));
    }
    Ok(())
}

pub(super) fn validate_all_configs_export(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_all_configs_export_header(incoming)?;
    validate_llm_rows(incoming)?;
    validate_embedding_rows(incoming)?;
    validate_image_rows(incoming)?;
    validate_video_rows(incoming)?;
    validate_prompt_rows(incoming)?;
    validate_theme_rows(incoming)?;
    validate_system_setting_rows(incoming)?;
    Ok(())
}

pub(super) fn normalize_all_configs_export(incoming: &mut AllConfigsExport) -> Result<(), String> {
    for (index, row) in incoming.llm_configs.iter_mut().enumerate() {
        let label = format!("LLM 配置 #{}", index + 1);
        trim_string(&mut row.name);
        trim_string(&mut row.provider);
        trim_string(&mut row.base_url);
        trim_string(&mut row.model);
        row.request_options_json = normalize_request_options_json(row.request_options_json.clone())
            .map_err(|err| {
                format!(
                    "{label} request_options_json 规范化失败：{}",
                    plain_error(err)
                )
            })?;
    }

    for (index, row) in incoming.embedding_configs.iter_mut().enumerate() {
        let params = validate_embedding_config_params(embedding_params_from_row(row))
            .map_err(|err| format!("嵌入配置 #{} 规范化失败：{err}", index + 1))?;
        apply_embedding_params_to_row(row, params);
    }

    for (index, row) in incoming.image_configs.iter_mut().enumerate() {
        let params = validate_image_config_params(image_params_from_row(row))
            .map_err(|err| format!("图片配置 #{} 规范化失败：{err}", index + 1))?;
        apply_image_params_to_row(row, params);
    }

    for (index, row) in incoming.video_configs.iter_mut().enumerate() {
        let params = validate_video_config_params(video_params_from_row(row))
            .map_err(|err| format!("视频配置 #{} 规范化失败：{err}", index + 1))?;
        apply_video_params_to_row(row, params);
    }

    for row in &mut incoming.prompts_modified {
        trim_string(&mut row.name);
    }

    for row in &mut incoming.themes {
        trim_string(&mut row.name);
        trim_string(&mut row.mode);
    }

    for row in &mut incoming.system_settings {
        trim_string(&mut row.key);
        trim_string(&mut row.category);
    }
    Ok(())
}

fn validate_llm_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "LLM 配置",
        &incoming.llm_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            required_text(&row.provider, label, "provider")?;
            required_text(&row.base_url, label, "base_url")?;
            required_text(&row.model, label, "model")?;
            positive_i32(row.max_concurrent, label, "max_concurrent")?;
            positive_i32(row.timeout_secs, label, "timeout_secs")?;
            positive_i32(row.max_context_tokens, label, "max_context_tokens")?;
            positive_i32(row.max_output_tokens, label, "max_output_tokens")?;
            if row.max_output_tokens >= row.max_context_tokens {
                return Err(format!(
                    "{label} max_output_tokens 必须小于 max_context_tokens"
                ));
            }
            validate_llm_request_options(row, label)?;
            Ok(())
        },
    )
}

fn validate_llm_request_options(row: &LlmRow, label: &str) -> Result<(), String> {
    normalize_request_options_json(row.request_options_json.clone())
        .map(|_| ())
        .map_err(|err| format!("{label} request_options_json 无效：{}", plain_error(err)))
}

fn plain_error(err: AppError) -> String {
    match err {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        other => other.to_string(),
    }
}

fn validate_embedding_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "嵌入配置",
        &incoming.embedding_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            validate_embedding_config_params(embedding_params_from_row(row))
                .map(|_| ())
                .map_err(|err| format!("{label} {err}"))
        },
    )
}

fn validate_image_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "图片配置",
        &incoming.image_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            validate_image_config_params(image_params_from_row(row))
                .map(|_| ())
                .map_err(|err| format!("{label} {err}"))
        },
    )
}

fn validate_video_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "视频配置",
        &incoming.video_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            validate_video_config_params(video_params_from_row(row))
                .map(|_| ())
                .map_err(|err| format!("{label} {err}"))
        },
    )
}

fn validate_default_named_rows<T>(
    kind: &str,
    rows: &[T],
    is_default: impl Fn(&T) -> bool,
    name: impl for<'a> Fn(&'a T) -> &'a str,
    validate: impl Fn(&str, &T) -> Result<(), String>,
) -> Result<(), String> {
    ensure_at_most_one_default(kind, rows.iter().filter(|row| is_default(*row)).count())?;

    let mut seen_names = HashSet::new();
    let identity_kind = format!("{kind} name");
    for (index, row) in rows.iter().enumerate() {
        let label = format!("{kind} #{}", index + 1);
        let name = required_text(name(row), &label, "name")?;
        ensure_unique_identity(&mut seen_names, &identity_kind, name)?;
        validate(&label, row)?;
    }
    Ok(())
}

fn validate_prompt_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    let mut seen_names = HashSet::new();
    for (index, row) in incoming.prompts_modified.iter().enumerate() {
        let label = format!("提示词 #{}", index + 1);
        let name = required_text(&row.name, &label, "name")?;
        ensure_unique_identity(&mut seen_names, "提示词 name", name)?;
        required_text(&row.content, &label, "content")?;
    }
    Ok(())
}

fn validate_theme_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    ensure_at_most_one_default(
        "系统默认主题",
        incoming
            .themes
            .iter()
            .filter(|row| row.is_system_default)
            .count(),
    )?;

    let mut seen_keys = HashSet::new();
    let mut active_scopes = HashSet::new();
    for (index, row) in incoming.themes.iter().enumerate() {
        let label = format!("主题 #{}", index + 1);
        let name = required_text(&row.name, &label, "name")?;
        required_text(&row.mode, &label, "mode")?;
        let scope = row
            .user_id
            .map(|user_id| user_id.to_string())
            .unwrap_or_else(|| "system".to_string());
        ensure_unique_identity(
            &mut seen_keys,
            "主题 user_id/name",
            format!("{scope}:{name}"),
        )?;
        if row.is_active {
            ensure_unique_identity(&mut active_scopes, "主题 active user_id", &scope)?;
        }
    }
    Ok(())
}

fn validate_system_setting_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    let mut seen_keys = HashSet::new();
    for (index, row) in incoming.system_settings.iter().enumerate() {
        let label = format!("系统设置 #{}", index + 1);
        let key = required_text(&row.key, &label, "key")?;
        ensure_unique_identity(&mut seen_keys, "系统设置 key", key)?;
        required_text(&row.category, &label, "category")?;
        if key.trim() == crate::dynamic_llm::LLM_MAX_TOKENS_SETTING_KEY {
            validate_llm_max_tokens_setting(incoming, &label, row.value.clone())?;
        }
    }
    Ok(())
}

fn validate_llm_max_tokens_setting(
    incoming: &AllConfigsExport,
    label: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let config: LlmMaxTokensConfig = serde_json::from_value(value)
        .map_err(|err| format!("{label} llm_max_tokens 解析失败：{err}"))?;
    config
        .validate()
        .map_err(|err| format!("{label} llm_max_tokens 无效：{err}"))?;
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
            .map_err(|err| format!("{label} llm_max_tokens 无效：{err}"))?;
    }
    Ok(())
}

fn ensure_at_most_one_default(kind: &str, count: usize) -> Result<(), String> {
    if count > 1 {
        return Err(format!("{kind}最多只能有一个默认项，当前为 {count} 个"));
    }
    Ok(())
}

fn ensure_unique_identity(
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

fn required_text<'a>(value: &'a str, label: &str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} 缺少 {field}"));
    }
    Ok(value)
}

fn positive_i32(value: i32, label: &str, field: &str) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("{label} {field} 必须为正整数"));
    }
    Ok(())
}

fn trim_string(value: &mut String) {
    let trimmed = value.trim();
    if trimmed.len() != value.len() {
        *value = trimmed.to_string();
    }
}

fn embedding_params_from_row(row: &EmbeddingRow) -> CreateEmbeddingConfigParams {
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

fn apply_embedding_params_to_row(row: &mut EmbeddingRow, params: CreateEmbeddingConfigParams) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = params.base_url;
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.dimension = params.dimension;
    row.batch_size = params.batch_size;
    row.local_model_path = params.local_model_path;
}

fn image_params_from_row(row: &ImageRow) -> CreateImageConfigParams {
    CreateImageConfigParams {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone().unwrap_or_default(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        max_concurrent: row.max_concurrent,
        default_width: row.default_width,
        default_height: row.default_height,
        provider_options_json: row.provider_options_json.clone(),
    }
}

fn apply_image_params_to_row(row: &mut ImageRow, params: CreateImageConfigParams) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = Some(params.base_url);
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.max_concurrent = params.max_concurrent;
    row.default_width = params.default_width;
    row.default_height = params.default_height;
    row.provider_options_json = params.provider_options_json;
}

fn video_params_from_row(row: &VideoRow) -> VideoConfigParams {
    VideoConfigParams {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone().unwrap_or_default(),
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
    }
}

fn apply_video_params_to_row(row: &mut VideoRow, params: VideoConfigParams) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = Some(params.base_url);
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
    row.provider_options_json = params.provider_options_json;
}

#[cfg(test)]
mod tests;
