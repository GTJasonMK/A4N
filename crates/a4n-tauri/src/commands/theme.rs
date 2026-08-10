//! 主题配置命令

use a4n_runtime::AppState;
use a4n_runtime::host_api::theme as theme_actions;
use a4n_runtime::host_api::theme::{CreateThemeInput, ThemeSummary, UpdateThemeInput};
use tauri::State;

macro_rules! theme_command {
    ($name:ident($state:ident $(, $arg:ident: $ty:ty)*) -> $output:ty, $body:expr) => {
        #[tauri::command]
        pub async fn $name(
            $state: State<'_, AppState>,
            $($arg: $ty,)*
        ) -> Result<$output, String> {
            $body.await.map_err(|e| e.to_string())
        }
    };
}

theme_command!(
    list_themes(state, user_id: i64) -> Vec<ThemeSummary>,
    theme_actions::list_themes_for_user(&state, user_id)
);
theme_command!(
    create_theme(state, user_id: i64, input: CreateThemeInput) -> ThemeSummary,
    theme_actions::create_theme_for_user(&state, user_id, input)
);
theme_command!(
    update_theme(state, user_id: i64, theme_id: i64, input: UpdateThemeInput) -> ThemeSummary,
    theme_actions::update_theme_for_user(&state, user_id, theme_id, input)
);
theme_command!(
    delete_theme(state, user_id: i64, theme_id: i64) -> (),
    theme_actions::delete_theme_for_user(&state, user_id, theme_id)
);
theme_command!(
    activate_theme(state, user_id: i64, theme_id: i64) -> ThemeSummary,
    theme_actions::activate_theme_for_user(&state, user_id, theme_id)
);
theme_command!(
    export_theme(state, user_id: i64, theme_id: i64) -> String,
    theme_actions::export_theme_for_user(&state, user_id, theme_id)
);
theme_command!(
    import_theme(state, user_id: i64, input: CreateThemeInput) -> ThemeSummary,
    theme_actions::import_theme_for_user(&state, user_id, input)
);
theme_command!(
    get_theme_css_variables(state, user_id: i64, theme_id: i64) -> String,
    theme_actions::get_theme_css_variables_for_user(&state, user_id, theme_id)
);
