use a4n_api_contract::theme::{
    CreateThemeInput, DeleteThemeResourceInput, ThemePackageUploadInput, ThemeResourceUploadInput,
    UpdateThemeInput, UpdateThemePreferenceInput,
};
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
theme_web_command!(create_theme(state, user_id, args) {
    let input: CreateThemeInput = command_input(args, "create_theme")?;
    to_value(theme_actions::create_theme_for_user(state.runtime(), user_id, input).await?)
});
theme_web_command!(update_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    let input: UpdateThemeInput = command_input(args, "update_theme")?;
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
    let input: CreateThemeInput = command_input(args, "import_theme")?;
    to_value(theme_actions::import_theme_for_user(state.runtime(), user_id, input).await?)
});
theme_web_command!(get_theme_preference(state, user_id, _args) {
    to_value(theme_actions::get_theme_preference_for_user(state.runtime(), user_id).await?)
});
theme_web_command!(update_theme_preference(state, user_id, args) {
    let input: UpdateThemePreferenceInput = command_input(args, "update_theme_preference")?;
    to_value(
        theme_actions::update_theme_preference_for_user(state.runtime(), user_id, input).await?,
    )
});
theme_web_command!(upload_theme_resource(state, user_id, args) {
    let input: ThemeResourceUploadInput = command_input(args, "upload_theme_resource")?;
    to_value(
        theme_actions::upload_theme_resource_for_user(state.runtime(), user_id, input).await?,
    )
});
theme_web_command!(delete_theme_resource(state, user_id, args) {
    let input: DeleteThemeResourceInput = command_input(args, "delete_theme_resource")?;
    theme_actions::delete_theme_resource_for_user(state.runtime(), user_id, input).await?;
    Ok(Value::Null)
});
theme_web_command!(export_theme_package(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    to_value(
        theme_actions::export_theme_package_for_user(state.runtime(), user_id, theme_id).await?,
    )
});
theme_web_command!(import_theme_package(state, user_id, args) {
    let input: ThemePackageUploadInput = command_input(args, "import_theme_package")?;
    to_value(
        theme_actions::import_theme_package_for_user(state.runtime(), user_id, input).await?,
    )
});
