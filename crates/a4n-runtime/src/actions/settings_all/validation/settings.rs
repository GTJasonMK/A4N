use super::super::types::AllConfigsExport;
use super::shared::{
    ensure_at_most_one_default, ensure_unique_identity, required_text, trim_string,
};
use a4n_core::config::app_config::LlmMaxTokensConfig;
use std::collections::HashSet;

pub(super) fn validate_setting_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_prompt_rows(incoming)?;
    validate_theme_rows(incoming)?;
    validate_system_setting_rows(incoming)
}

pub(super) fn normalize_setting_rows(incoming: &mut AllConfigsExport) {
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
