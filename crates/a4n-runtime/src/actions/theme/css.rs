use super::access::ensure_theme_access;
use super::rules::ensure_theme_css_variables_object;
use crate::AppState;
use a4n_core::error::AppError;
use serde_json::Value;

pub async fn get_theme_css_variables_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    ensure_theme_access(state, user_id, theme_id).await?;
    get_theme_css_variables(state, theme_id).await
}

async fn get_theme_css_variables(state: &AppState, theme_id: i64) -> Result<String, AppError> {
    let theme = state
        .orchestrator
        .theme_by_id(theme_id)
        .await?
        .ok_or_else(|| AppError::NotFound("主题不存在".to_string()))?;
    ensure_theme_css_variables_object(&theme.css_variables).map_err(AppError::InvalidArgument)?;
    Ok(render_theme_css_variables(&theme.css_variables))
}

pub(super) fn render_theme_css_variables(css_variables: &Value) -> String {
    let mut css = String::from(":root {\n");
    if let Some(object) = css_variables.as_object() {
        for (key, value) in object {
            let value = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            css.push_str(&format!("  {}: {};\n", css_variable_name(key), value));
        }
    }
    css.push('}');
    css
}

fn css_variable_name(key: &str) -> String {
    if key.starts_with("--") {
        key.to_string()
    } else {
        format!("--{key}")
    }
}
