use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::models::user::theme_config;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const THEME_MODES: &[&str] = &["light", "dark"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSummary {
    pub id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: String,
    pub css_variables: Value,
    pub is_system_default: bool,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateThemeInput {
    pub name: String,
    pub mode: String,
    pub css_variables: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateThemeInput {
    pub name: Option<String>,
    pub mode: Option<String>,
    pub css_variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportThemeInput {
    pub theme_json: String,
}

#[derive(Debug, Deserialize)]
struct ThemeImportPayload {
    name: String,
    mode: String,
    css_variables: Value,
}

pub async fn list_themes(state: &AppState) -> Result<Vec<ThemeSummary>, String> {
    let themes = state.theme.list().await.map_err(|e| e.to_string())?;
    Ok(themes.iter().map(to_summary).collect())
}

pub async fn list_themes_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    let themes = state.theme.list_visible_to_user(user_id).await?;
    Ok(themes.iter().map(to_summary).collect())
}

async fn list_user_themes(state: &AppState, user_id: i64) -> Result<Vec<ThemeSummary>, String> {
    let themes = state
        .theme
        .list_by_user(user_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(themes.iter().map(to_summary).collect())
}

pub async fn list_user_themes_for_user(
    state: &AppState,
    user_id: i64,
    target_user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    ensure_same_user(user_id, target_user_id, "只能列出当前登录用户主题")?;
    list_user_themes(state, target_user_id)
        .await
        .map_err(AppError::InvalidArgument)
}

pub async fn get_system_default_theme(state: &AppState) -> Result<Option<ThemeSummary>, String> {
    let theme = state
        .theme
        .system_default()
        .await
        .map_err(|e| e.to_string())?;
    Ok(theme.as_ref().map(to_summary))
}

pub async fn create_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    create_theme(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn create_theme(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, String> {
    let name = normalize_theme_name(input.name)?;
    let mode = normalize_theme_mode(input.mode)?;
    ensure_theme_css_variables_object(&input.css_variables)?;
    let theme = state
        .theme
        .create(Some(user_id), &name, &mode, input.css_variables)
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_summary(&theme))
}

pub async fn update_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    input: UpdateThemeInput,
) -> Result<ThemeSummary, AppError> {
    ensure_theme_write_access(state, user_id, theme_id, "只能修改当前登录用户主题").await?;
    update_theme(state, theme_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn update_theme(
    state: &AppState,
    theme_id: i64,
    input: UpdateThemeInput,
) -> Result<ThemeSummary, String> {
    let name = input.name.map(normalize_theme_name).transpose()?;
    let mode = input.mode.map(normalize_theme_mode).transpose()?;
    if let Some(css_variables) = input.css_variables.as_ref() {
        ensure_theme_css_variables_object(css_variables)?;
    }
    let theme = state
        .theme
        .update(
            theme_id,
            name.as_deref(),
            mode.as_deref(),
            input.css_variables,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_summary(&theme))
}

pub async fn delete_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    ensure_theme_write_access(state, user_id, theme_id, "只能删除当前登录用户主题").await?;
    delete_theme(state, theme_id)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn delete_theme(state: &AppState, theme_id: i64) -> Result<(), String> {
    let theme = theme_config::Entity::find_by_id(theme_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "主题不存在".to_string())?;
    ensure_theme_can_be_deleted(theme.is_system_default, theme.is_active)?;
    state
        .theme
        .delete(theme_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn activate_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<ThemeSummary, AppError> {
    ensure_theme_write_access(state, user_id, theme_id, "只能激活当前登录用户主题").await?;
    activate_theme(state, theme_id)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn activate_theme(state: &AppState, theme_id: i64) -> Result<ThemeSummary, String> {
    let txn = state.db.begin().await.map_err(|e| e.to_string())?;
    let target = theme_config::Entity::find_by_id(theme_id)
        .one(&txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "主题不存在".to_string())?;
    let user_id = target.user_id;
    let now = chrono::Utc::now().naive_utc();

    let mut clear_scope = theme_config::Entity::update_many()
        .col_expr(
            theme_config::Column::IsActive,
            sea_orm::sea_query::Expr::value(false),
        )
        .col_expr(
            theme_config::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        );
    clear_scope = match user_id {
        Some(uid) => clear_scope.filter(theme_config::Column::UserId.eq(uid)),
        None => clear_scope.filter(theme_config::Column::UserId.is_null()),
    };
    clear_scope.exec(&txn).await.map_err(|e| e.to_string())?;

    let mut active: theme_config::ActiveModel = target.into();
    active.is_active = Set(true);
    active.updated_at = Set(now);
    let updated = active.update(&txn).await.map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(to_summary(&updated))
}

pub async fn export_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    ensure_theme_access(state, user_id, theme_id).await?;
    export_theme(state, theme_id)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn export_theme(state: &AppState, theme_id: i64) -> Result<String, String> {
    let theme = theme_config::Entity::find_by_id(theme_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "主题不存在".to_string())?;

    serde_json::to_string_pretty(&to_summary(&theme)).map_err(|e| e.to_string())
}

pub async fn import_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: ImportThemeInput,
) -> Result<ThemeSummary, AppError> {
    import_theme(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn import_theme(
    state: &AppState,
    user_id: i64,
    input: ImportThemeInput,
) -> Result<ThemeSummary, String> {
    let incoming: ThemeImportPayload =
        serde_json::from_str(&input.theme_json).map_err(|e| format!("JSON 解析失败: {e}"))?;
    let name = normalize_theme_name(incoming.name)?;
    let mode = normalize_theme_mode(incoming.mode)?;
    ensure_theme_css_variables_object(&incoming.css_variables)?;
    let theme = state
        .theme
        .create(Some(user_id), &name, &mode, incoming.css_variables)
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_summary(&theme))
}

pub async fn get_theme_css_variables_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    ensure_theme_access(state, user_id, theme_id).await?;
    get_theme_css_variables(state, theme_id)
        .await
        .map_err(AppError::InvalidArgument)
}

async fn get_theme_css_variables(state: &AppState, theme_id: i64) -> Result<String, String> {
    let theme = theme_config::Entity::find_by_id(theme_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "主题不存在".to_string())?;
    ensure_theme_css_variables_object(&theme.css_variables)?;

    Ok(render_theme_css_variables(&theme.css_variables))
}

async fn theme_user_id(state: &AppState, theme_id: i64) -> Result<Option<i64>, String> {
    let theme = theme_config::Entity::find_by_id(theme_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "主题不存在".to_string())?;
    Ok(theme.user_id)
}

async fn ensure_theme_access(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    let theme_user_id = theme_user_id(state, theme_id)
        .await
        .map_err(AppError::InvalidArgument)?;
    if let Some(owner_id) = theme_user_id {
        ensure_same_user(user_id, owner_id, "只能访问当前登录用户主题")?;
    }
    Ok(())
}

async fn ensure_theme_write_access(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    message: &str,
) -> Result<(), AppError> {
    let theme_user_id = theme_user_id(state, theme_id)
        .await
        .map_err(AppError::InvalidArgument)?;
    let Some(owner_id) = theme_user_id else {
        return Err(AppError::Forbidden(message.to_string()));
    };
    ensure_same_user(user_id, owner_id, message)
}

fn ensure_same_user(user_id: i64, target_user_id: i64, message: &str) -> Result<(), AppError> {
    if target_user_id != user_id {
        return Err(AppError::Forbidden(message.to_string()));
    }
    Ok(())
}

fn normalize_theme_name(name: String) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("主题名称不能为空".to_string());
    }
    Ok(name.to_string())
}

fn normalize_theme_mode(mode: String) -> Result<String, String> {
    let mode = mode.trim();
    if !THEME_MODES.contains(&mode) {
        return Err(format!(
            "主题 mode 无效：{mode}，只支持 {}",
            THEME_MODES.join("、")
        ));
    }
    Ok(mode.to_string())
}

fn ensure_theme_css_variables_object(css_variables: &Value) -> Result<(), String> {
    if !css_variables.is_object() {
        return Err("主题 css_variables 必须是 JSON 对象".to_string());
    }
    Ok(())
}

fn ensure_theme_can_be_deleted(is_system_default: bool, is_active: bool) -> Result<(), String> {
    if is_system_default {
        return Err("系统默认主题不能删除".to_string());
    }
    if is_active {
        return Err("正在使用的主题不能删除，请先激活其他主题".to_string());
    }
    Ok(())
}

fn to_summary(theme: &theme_config::Model) -> ThemeSummary {
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

fn css_variable_name(key: &str) -> String {
    if key.starts_with("--") {
        key.to_string()
    } else {
        format!("--{key}")
    }
}

fn render_theme_css_variables(css_variables: &Value) -> String {
    let mut css = String::from(":root {\n");
    if let Some(obj) = css_variables.as_object() {
        for (key, val) in obj {
            let value = match val {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            css.push_str(&format!("  {}: {};\n", css_variable_name(key), value));
        }
    }
    css.push('}');
    css
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_theme_css_variables_preserves_existing_custom_property_prefix() {
        let css = render_theme_css_variables(&json!({
            "--color-primary": "139 69 19",
            "color-secondary": "255 255 255",
            "radius": 8,
        }));

        assert!(css.contains("  --color-primary: 139 69 19;"));
        assert!(css.contains("  --color-secondary: 255 255 255;"));
        assert!(css.contains("  --radius: 8;"));
    }

    #[test]
    fn import_payload_accepts_exported_theme_summary_shape() {
        let payload: ThemeImportPayload = serde_json::from_value(json!({
            "id": 1,
            "user_id": 7,
            "name": "Dark",
            "mode": "dark",
            "css_variables": {"color": "black"},
            "is_system_default": false,
            "is_active": false,
            "created_at": "2026-01-01 00:00:00",
            "updated_at": "2026-01-01 00:00:00"
        }))
        .expect("exported theme summary should be importable");

        assert_eq!(payload.name, "Dark");
        assert_eq!(payload.mode, "dark");
        assert_eq!(payload.css_variables, json!({"color": "black"}));
    }

    #[test]
    fn normalize_theme_name_trims_and_rejects_empty() {
        assert_eq!(
            normalize_theme_name("  Quiet Dark  ".to_string()).unwrap(),
            "Quiet Dark"
        );

        let err = normalize_theme_name("   ".to_string())
            .expect_err("empty theme names should fail at command boundary");
        assert!(err.contains("主题名称不能为空"));
    }

    #[test]
    fn normalize_theme_mode_accepts_only_supported_modes() {
        assert_eq!(normalize_theme_mode(" dark ".to_string()).unwrap(), "dark");
        assert_eq!(normalize_theme_mode("light".to_string()).unwrap(), "light");

        let err = normalize_theme_mode("system".to_string())
            .expect_err("unsupported theme mode should fail explicitly");
        assert!(err.contains("只支持 light、dark"));
    }

    #[test]
    fn theme_css_variables_must_be_object() {
        ensure_theme_css_variables_object(&json!({
            "color-primary": "139 69 19"
        }))
        .expect("object css_variables should pass");

        let err = ensure_theme_css_variables_object(&json!(["not", "object"]))
            .expect_err("array css_variables should fail explicitly");
        assert!(err.contains("css_variables 必须是 JSON 对象"));
    }

    #[test]
    fn delete_theme_rejects_system_default_or_active_theme() {
        let err = ensure_theme_can_be_deleted(true, false)
            .expect_err("system default theme should not be deletable");
        assert!(err.contains("系统默认主题不能删除"));

        let err = ensure_theme_can_be_deleted(false, true)
            .expect_err("active theme should not be deletable");
        assert!(err.contains("正在使用的主题不能删除"));

        ensure_theme_can_be_deleted(false, false).expect("inactive non-system theme can delete");
    }

    #[test]
    fn ensure_same_user_rejects_cross_user_theme_access() {
        let err = ensure_same_user(7, 9, "只能访问当前登录用户主题")
            .expect_err("cross-user theme access should be rejected");

        assert!(matches!(err, AppError::Forbidden(_)));
        assert!(err.to_string().contains("只能访问当前登录用户主题"));
    }

    #[test]
    fn create_theme_input_rejects_legacy_nested_user_id() {
        let err = serde_json::from_value::<CreateThemeInput>(json!({
            "user_id": 7,
            "name": "Dark",
            "mode": "dark",
            "css_variables": {"color-primary": "139 69 19"}
        }))
        .expect_err("theme owner must come from authenticated user, not nested input");

        assert!(err.to_string().contains("unknown field `user_id`"));
    }

    #[test]
    fn import_theme_input_rejects_legacy_nested_user_id() {
        let err = serde_json::from_value::<ImportThemeInput>(json!({
            "user_id": 7,
            "theme_json": "{}"
        }))
        .expect_err("theme import owner must come from authenticated user, not nested input");

        assert!(err.to_string().contains("unknown field `user_id`"));
    }
}
