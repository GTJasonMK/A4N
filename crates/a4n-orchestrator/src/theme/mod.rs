//! 主题配置业务规则与权限门禁。

use std::collections::HashSet;

mod rules;

use a4n_api_contract::theme::{CreateThemeInput, UpdateThemeInput};
use a4n_core::{
    error::AppError,
    models::user::{theme_config, theme_preference, theme_resource},
};
use a4n_domain::ThemeAppearance;

use crate::{OrchestratorContext, auth::ensure_same_user};
use rules::{ensure_theme_can_be_deleted, normalize_theme_name};

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
        a4n_domain::validate_theme_document(&input.document).map_err(AppError::InvalidArgument)?;
        self.create_user_theme_record(user_id, name, input.mode, input.document)
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
            .as_ref()
            .map(|value| normalize_theme_name(value.clone()))
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        if let Some(document) = input.document {
            a4n_domain::validate_theme_document(&document).map_err(AppError::InvalidArgument)?;
            self.update_theme_record(theme, name, input.mode, Some(document))
                .await
        } else {
            self.update_theme_record(theme, name, input.mode, None)
                .await
        }
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
        mut input: CreateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        let name = normalize_theme_name(input.name).map_err(AppError::InvalidArgument)?;
        let existing_names = self
            .list_themes_visible_to_user(user_id)
            .await?
            .into_iter()
            .filter(|theme| theme.user_id == Some(user_id))
            .map(|theme| theme.name)
            .collect::<HashSet<_>>();
        if existing_names.contains(&name) {
            let mut suffix = 1;
            let mut candidate = format!("{name} (导入)");
            while existing_names.contains(&candidate) {
                suffix += 1;
                candidate = format!("{name} (导入 {suffix})");
            }
            input.name = candidate;
        } else {
            input.name = name;
        }
        self.create_theme_for_user_validated(user_id, input).await
    }

    pub async fn list_theme_resources_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<Vec<theme_resource::Model>, AppError> {
        self.require_theme_access(user_id, theme_id).await?;
        self.list_theme_resources(theme_id).await
    }

    pub async fn upsert_theme_resource_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
        resource_key: &str,
        file_name: &str,
        mime_type: &str,
        file_path: &str,
        sha256: &str,
        byte_size: i64,
    ) -> Result<theme_resource::Model, AppError> {
        self.require_theme_owner(user_id, theme_id, "只能修改当前登录用户主题")
            .await?;
        self.upsert_theme_resource(
            theme_id,
            resource_key,
            file_name,
            mime_type,
            file_path,
            sha256,
            byte_size,
        )
        .await
    }

    pub async fn delete_theme_resource_for_user_validated(
        &self,
        user_id: i64,
        resource_id: i64,
    ) -> Result<theme_resource::Model, AppError> {
        let resource = self
            .theme_resource_by_id(resource_id)
            .await?
            .ok_or_else(|| AppError::NotFound("主题资源不存在".to_string()))?;
        self.require_theme_owner(user_id, resource.theme_id, "只能删除当前登录用户主题")
            .await?;
        self.delete_theme_resource(resource_id).await
    }

    pub async fn get_theme_preference_for_user_validated(
        &self,
        user_id: i64,
    ) -> Result<Option<theme_preference::Model>, AppError> {
        self.theme_preference(user_id).await
    }

    pub async fn update_theme_preference_for_user_validated(
        &self,
        user_id: i64,
        appearance: ThemeAppearance,
    ) -> Result<theme_preference::Model, AppError> {
        self.upsert_theme_preference(user_id, appearance).await
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
