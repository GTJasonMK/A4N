use super::ThemeSummary;
use a4n_core::models::user::theme_config;

pub(super) fn to_summary(theme: &theme_config::Model) -> ThemeSummary {
    ThemeSummary {
        id: theme.id,
        user_id: theme.user_id,
        name: theme.name.clone(),
        mode: theme.mode.clone(),
        css_variables: theme.css_variables.clone(),
        is_system_default: theme.is_system_default,
        is_active: theme.is_active,
        created_at: Some(theme.created_at.to_string()),
        updated_at: Some(theme.updated_at.to_string()),
    }
}
