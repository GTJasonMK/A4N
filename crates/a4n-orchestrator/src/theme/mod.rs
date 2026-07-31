//! 主题配置：纯规则（DB 行为经 settings facade / context settings）。

mod rules;

use a4n_core::{error::AppError, models::user::theme_config};
use serde::Deserialize;
use serde_json::Value;

use crate::OrchestratorContext;

/// 同用户门禁与 auth 共用，避免双实现。
pub use crate::auth::ensure_same_user;
pub use rules::{
    ensure_theme_can_be_deleted, ensure_theme_css_variables_object, normalize_theme_mode,
    normalize_theme_name, render_theme_css_variables,
};

pub use a4n_api_contract::theme::{CreateThemeInput, ImportThemeInput, UpdateThemeInput};

#[derive(Debug, Deserialize)]
struct ThemeImportPayload {
    name: String,
    mode: String,
    css_variables: Value,
}

impl OrchestratorContext {
    pub async fn list_themes_for_user_validated(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.list_themes_visible_to_user(user_id).await
    }

    pub async fn list_user_themes_for_user_validated(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        ensure_same_user(caller_user_id, target_user_id, "只能列出当前登录用户主题")?;
        self.list_user_themes(target_user_id).await
    }

    pub async fn create_theme_for_user_validated(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        let name = normalize_theme_name(input.name).map_err(AppError::InvalidArgument)?;
        let mode = normalize_theme_mode(input.mode).map_err(AppError::InvalidArgument)?;
        ensure_theme_css_variables_object(&input.css_variables)
            .map_err(AppError::InvalidArgument)?;
        self.create_user_theme(user_id, name, mode, input.css_variables)
            .await
    }

    pub async fn update_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
        input: UpdateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        self.require_theme_owner(user_id, theme_id, "只能修改当前登录用户主题")
            .await?;
        let name = input
            .name
            .map(normalize_theme_name)
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        let mode = input
            .mode
            .map(normalize_theme_mode)
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        if let Some(css_variables) = input.css_variables.as_ref() {
            ensure_theme_css_variables_object(css_variables).map_err(AppError::InvalidArgument)?;
        }
        self.update_theme_fields(theme_id, name, mode, input.css_variables)
            .await
    }

    pub async fn delete_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<(), AppError> {
        self.require_theme_owner(user_id, theme_id, "只能删除当前登录用户主题")
            .await?;
        self.delete_theme_if_allowed(theme_id).await
    }

    pub async fn activate_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        self.require_theme_owner(user_id, theme_id, "只能激活当前登录用户主题")
            .await?;
        self.activate_theme(theme_id).await
    }

    pub async fn export_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        self.require_theme_access(user_id, theme_id).await
    }

    pub async fn import_theme_for_user_validated(
        &self,
        user_id: i64,
        input: ImportThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        let payload: ThemeImportPayload = serde_json::from_str(&input.theme_json)
            .map_err(|error| AppError::InvalidArgument(format!("JSON 解析失败: {error}")))?;
        self.create_theme_for_user_validated(
            user_id,
            CreateThemeInput {
                name: payload.name,
                mode: payload.mode,
                css_variables: payload.css_variables,
            },
        )
        .await
    }

    pub async fn theme_css_variables_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<String, AppError> {
        let theme = self.require_theme_access(user_id, theme_id).await?;
        ensure_theme_css_variables_object(&theme.css_variables)
            .map_err(AppError::InvalidArgument)?;
        Ok(render_theme_css_variables(&theme.css_variables))
    }

    async fn require_theme_access(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        let theme = self
            .theme_by_id(theme_id)
            .await?
            .ok_or_else(|| AppError::NotFound("主题不存在".to_string()))?;
        if let Some(owner_id) = theme.user_id {
            ensure_same_user(user_id, owner_id, "只能访问当前登录用户主题")?;
        }
        Ok(theme)
    }

    async fn require_theme_owner(
        &self,
        user_id: i64,
        theme_id: i64,
        message: &str,
    ) -> Result<theme_config::Model, AppError> {
        let theme = self.require_theme_access(user_id, theme_id).await?;
        if theme.user_id.is_none() {
            return Err(AppError::Forbidden(message.to_string()));
        }
        Ok(theme)
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeImportPayload;

    #[test]
    fn imports_exported_theme_summary_fields() {
        let payload: ThemeImportPayload = serde_json::from_str(
            r#"{
                "id": 1,
                "user_id": 7,
                "name": "Dark",
                "mode": "dark",
                "css_variables": {},
                "is_system_default": false,
                "is_active": false,
                "created_at": "2026-07-24T00:00:00Z",
                "updated_at": "2026-07-24T00:00:00Z"
            }"#,
        )
        .expect("exported theme summary remains importable");

        assert_eq!(payload.name, "Dark");
        assert_eq!(payload.mode, "dark");
    }
}
