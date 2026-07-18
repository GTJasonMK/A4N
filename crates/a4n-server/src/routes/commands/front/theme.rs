use serde_json::{Value, json};

use super::super::{ApiError, ServerState, command_input, i64_arg, theme_actions, to_value};

macro_rules! theme_web_command {
    ($name:ident($state:ident, $user_id:ident, $args:ident) $body:block) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> $body
    };
}

theme_web_command!(list_themes(state, user_id, _args) {
    to_value(theme_actions::list_themes_for_user(state.runtime(), user_id).await?)
});
theme_web_command!(get_system_default_theme(state, _user_id, _args) {
    to_value(
        theme_actions::get_system_default_theme(state.runtime())
            .await
            .map_err(ApiError::bad_request)?,
    )
});
theme_web_command!(list_user_themes(state, user_id, args) {
    let target_user_id = i64_arg(args, "userId")?;
    let themes =
        theme_actions::list_user_themes_for_user(state.runtime(), user_id, target_user_id).await?;
    to_value(themes)
});
theme_web_command!(create_theme(state, user_id, args) {
    let input: theme_actions::CreateThemeInput = command_input(args, "create_theme")?;
    to_value(theme_actions::create_theme_for_user(state.runtime(), user_id, input).await?)
});
theme_web_command!(update_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    let input: theme_actions::UpdateThemeInput = command_input(args, "update_theme")?;
    to_value(
        theme_actions::update_theme_for_user(state.runtime(), user_id, theme_id, input).await?,
    )
});
theme_web_command!(delete_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    theme_actions::delete_theme_for_user(state.runtime(), user_id, theme_id).await?;
    Ok(Value::Null)
});
theme_web_command!(activate_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    to_value(theme_actions::activate_theme_for_user(state.runtime(), user_id, theme_id).await?)
});
theme_web_command!(export_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    Ok(json!(
        theme_actions::export_theme_for_user(state.runtime(), user_id, theme_id).await?
    ))
});
theme_web_command!(import_theme(state, user_id, args) {
    let input: theme_actions::ImportThemeInput = command_input(args, "import_theme")?;
    to_value(theme_actions::import_theme_for_user(state.runtime(), user_id, input).await?)
});
theme_web_command!(get_theme_css_variables(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    Ok(json!(
        theme_actions::get_theme_css_variables_for_user(state.runtime(), user_id, theme_id).await?
    ))
});

pub(in crate::routes::commands) async fn export_all_configs_json(
    state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(
        a4n_runtime::host_api::settings_all::export_all_configs_to_json(state.runtime()).await?
    ))
}

pub(in crate::routes::commands) async fn import_all_configs(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: a4n_runtime::host_api::settings_all::ImportAllConfigsInput =
        command_input(args, "import_all_configs")?;
    Ok(json!(
        a4n_runtime::host_api::settings_all::import_all_configs(state.runtime(), input).await?
    ))
}
