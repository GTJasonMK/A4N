use super::ThemeSummary;
use a4n_api_contract::theme::ThemeCssVariables;
use a4n_core::{error::AppError, models::user::theme_config};
use a4n_domain::ThemeMode;

pub(super) fn to_summary(theme: theme_config::Model) -> Result<ThemeSummary, AppError> {
    let mode = theme.mode.parse::<ThemeMode>().map_err(|error| {
        AppError::ConfigError(format!("主题 {} 的持久化 mode 无效：{error}", theme.id))
    })?;
    let css_variables =
        serde_json::from_value::<ThemeCssVariables>(theme.css_variables).map_err(|error| {
            AppError::ConfigError(format!(
                "主题 {} 的持久化 css_variables 无效：{error}",
                theme.id
            ))
        })?;
    Ok(ThemeSummary {
        id: theme.id,
        user_id: theme.user_id,
        name: theme.name,
        mode,
        css_variables,
        is_system_default: theme.is_system_default,
        is_active: theme.is_active,
    })
}
