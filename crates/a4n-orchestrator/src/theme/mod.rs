//! 主题配置业务规则与权限门禁。

mod rules;

use a4n_core::{error::AppError, models::user::theme_config};

use crate::OrchestratorContext;

/// 同用户门禁与 auth 共用，避免双实现。
pub use crate::auth::ensure_same_user;
pub use rules::{ensure_theme_can_be_deleted, normalize_theme_name};

pub use a4n_api_contract::theme::{CreateThemeInput, UpdateThemeInput};

impl OrchestratorContext {
    pub async fn list_themes_for_user_validated(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.list_themes_visible_to_user(user_id).await
    }

    pub async fn create_theme_for_user_validated(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        let name = normalize_theme_name(input.name).map_err(AppError::InvalidArgument)?;
        self.create_user_theme_record(user_id, name, input.mode, input.css_variables)
            .await
    }

    pub async fn update_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
        input: UpdateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        if input.is_empty() {
            return Err(AppError::InvalidArgument(
                "主题更新至少需要一个字段".to_string(),
            ));
        }
        let theme = self
            .require_theme_owner(user_id, theme_id, "只能修改当前登录用户主题")
            .await?;
        let name = input
            .name
            .map(normalize_theme_name)
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        self.update_theme_record(theme, name, input.mode, input.css_variables)
            .await
    }

    pub async fn delete_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<(), AppError> {
        let theme = self
            .require_theme_owner(user_id, theme_id, "只能删除当前登录用户主题")
            .await?;
        ensure_theme_can_be_deleted(theme.is_system_default, theme.is_active)
            .map_err(AppError::InvalidArgument)?;
        self.delete_theme_record(theme.id).await
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
        input: CreateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        self.create_theme_for_user_validated(user_id, input).await
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
