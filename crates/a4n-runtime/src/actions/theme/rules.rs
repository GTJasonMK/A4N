use a4n_core::error::AppError;
use serde_json::Value;

const THEME_MODES: &[&str] = &["light", "dark"];

pub(super) fn ensure_same_user(
    user_id: i64,
    target_user_id: i64,
    message: &str,
) -> Result<(), AppError> {
    if target_user_id != user_id {
        return Err(AppError::Forbidden(message.to_string()));
    }
    Ok(())
}

pub(super) fn normalize_theme_name(name: String) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("主题名称不能为空".to_string());
    }
    Ok(name.to_string())
}

pub(super) fn normalize_theme_mode(mode: String) -> Result<String, String> {
    let mode = mode.trim();
    if !THEME_MODES.contains(&mode) {
        return Err(format!(
            "主题 mode 无效：{mode}，只支持 {}",
            THEME_MODES.join("、")
        ));
    }
    Ok(mode.to_string())
}

pub(super) fn ensure_theme_css_variables_object(css_variables: &Value) -> Result<(), String> {
    if !css_variables.is_object() {
        return Err("主题 css_variables 必须是 JSON 对象".to_string());
    }
    Ok(())
}

pub(super) fn ensure_theme_can_be_deleted(
    is_system_default: bool,
    is_active: bool,
) -> Result<(), String> {
    if is_system_default {
        return Err("系统默认主题不能删除".to_string());
    }
    if is_active {
        return Err("正在使用的主题不能删除，请先激活其他主题".to_string());
    }
    Ok(())
}
