//! 主题配置业务规则与权限门禁。

use std::collections::HashSet;

mod rules;

use a4n_api_contract::theme::{
    CreateThemeInput, PersistThemeResourceInput, ThemeResourceRecord, ThemeResourceSummary,
    ThemeSummary, UpdateThemeInput,
};
use a4n_db_models::models::user::{theme_config, theme_preference, theme_resource};
use a4n_domain::{ThemeAppearance, ThemeDocument, ThemeMode};
use a4n_error::AppError;

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

#[async_trait::async_trait]
impl a4n_api_contract::theme::ThemeReadApplicationPort for OrchestratorContext {
    async fn list_themes(&self, user_id: i64) -> Result<Vec<ThemeSummary>, AppError> {
        let themes = self.list_themes_visible_to_user(user_id).await?;
        let theme_ids = themes.iter().map(|theme| theme.id).collect::<Vec<_>>();
        let resources = self.list_theme_resources_for_themes(&theme_ids).await?;
        let mut resources_by_theme = std::collections::HashMap::<i64, Vec<_>>::new();
        for resource in resources {
            resources_by_theme
                .entry(resource.theme_id)
                .or_default()
                .push(resource);
        }
        themes
            .into_iter()
            .map(|theme| {
                let theme_id = theme.id;
                theme_summary_with_resources(
                    theme,
                    resources_by_theme.remove(&theme_id).unwrap_or_default(),
                )
            })
            .collect()
    }

    async fn export_theme(&self, user_id: i64, theme_id: i64) -> Result<ThemeSummary, AppError> {
        let theme = self
            .export_theme_for_user_validated(user_id, theme_id)
            .await?;
        self.theme_summary(user_id, theme).await
    }

    async fn list_resources(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<Vec<ThemeResourceRecord>, AppError> {
        self.list_theme_resources_for_user_validated(user_id, theme_id)
            .await
            .map(|resources| resources.into_iter().map(theme_resource_record).collect())
    }

    async fn get_preference(
        &self,
        user_id: i64,
    ) -> Result<a4n_api_contract::theme::ThemePreferenceSummary, AppError> {
        let appearance = self
            .get_theme_preference_for_user_validated(user_id)
            .await?
            .map(|preference| preference.appearance.parse::<ThemeAppearance>())
            .transpose()
            .map_err(|error| AppError::ConfigError(format!("主题外观偏好无效：{error}")))?
            .unwrap_or(ThemeAppearance::System);
        Ok(a4n_api_contract::theme::ThemePreferenceSummary {
            user_id,
            appearance,
        })
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::theme::ThemeMutationApplicationPort for OrchestratorContext {
    async fn create_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        let theme = self.create_theme_for_user_validated(user_id, input).await?;
        self.theme_summary(user_id, theme).await
    }

    async fn update_theme(
        &self,
        user_id: i64,
        theme_id: i64,
        input: UpdateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        let theme = self
            .update_theme_for_user_validated(user_id, theme_id, input)
            .await?;
        self.theme_summary(user_id, theme).await
    }

    async fn delete_theme(&self, user_id: i64, theme_id: i64) -> Result<(), AppError> {
        self.delete_theme_for_user_validated(user_id, theme_id)
            .await
    }

    async fn activate_theme(&self, user_id: i64, theme_id: i64) -> Result<ThemeSummary, AppError> {
        let theme = self
            .activate_theme_for_user_validated(user_id, theme_id)
            .await?;
        self.theme_summary(user_id, theme).await
    }

    async fn import_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        let theme = self.import_theme_for_user_validated(user_id, input).await?;
        self.theme_summary(user_id, theme).await
    }

    async fn upsert_resource(
        &self,
        user_id: i64,
        input: PersistThemeResourceInput,
    ) -> Result<ThemeResourceRecord, AppError> {
        let resource = self
            .upsert_theme_resource_for_user_validated(
                user_id,
                input.theme_id,
                &input.resource_key,
                &input.file_name,
                &input.mime_type,
                &input.file_path,
                &input.sha256,
                input.byte_size,
            )
            .await?;
        Ok(theme_resource_record(resource))
    }

    async fn delete_resource(
        &self,
        user_id: i64,
        resource_id: i64,
    ) -> Result<ThemeResourceRecord, AppError> {
        let resource = self
            .delete_theme_resource_for_user_validated(user_id, resource_id)
            .await?;
        Ok(theme_resource_record(resource))
    }

    async fn update_preference(
        &self,
        user_id: i64,
        input: a4n_api_contract::theme::UpdateThemePreferenceInput,
    ) -> Result<a4n_api_contract::theme::ThemePreferenceSummary, AppError> {
        let preference = self
            .update_theme_preference_for_user_validated(user_id, input.appearance)
            .await?;
        let appearance = preference
            .appearance
            .parse::<ThemeAppearance>()
            .map_err(|error| AppError::ConfigError(format!("主题外观偏好无效：{error}")))?;
        Ok(a4n_api_contract::theme::ThemePreferenceSummary {
            user_id,
            appearance,
        })
    }
}

impl OrchestratorContext {
    async fn theme_summary(
        &self,
        user_id: i64,
        theme: theme_config::Model,
    ) -> Result<ThemeSummary, AppError> {
        let resources = self
            .list_theme_resources_for_user_validated(user_id, theme.id)
            .await?;
        theme_summary_with_resources(theme, resources)
    }
}

fn theme_summary_with_resources(
    theme: theme_config::Model,
    resources: Vec<theme_resource::Model>,
) -> Result<ThemeSummary, AppError> {
    let mode = theme.mode.parse::<ThemeMode>().map_err(|error| {
        AppError::ConfigError(format!("主题 {} 的持久化 mode 无效：{error}", theme.id))
    })?;
    let document =
        serde_json::from_value::<ThemeDocument>(theme.theme_document.clone()).map_err(|error| {
            AppError::ConfigError(format!(
                "主题 {} 的持久化 theme_document 无效：{error}",
                theme.id
            ))
        })?;
    Ok(ThemeSummary {
        id: theme.id,
        user_id: theme.user_id,
        name: theme.name,
        mode,
        document,
        resources: resources
            .into_iter()
            .map(|resource| ThemeResourceSummary {
                id: resource.id,
                theme_id: resource.theme_id,
                resource_key: resource.resource_key,
                file_name: resource.file_name,
                mime_type: resource.mime_type,
                file_path: resource.file_path,
                byte_size: resource.byte_size,
            })
            .collect(),
        is_system_default: theme.is_system_default,
        is_active: theme.is_active,
    })
}

fn theme_resource_record(resource: theme_resource::Model) -> ThemeResourceRecord {
    ThemeResourceRecord {
        id: resource.id,
        theme_id: resource.theme_id,
        resource_key: resource.resource_key,
        file_name: resource.file_name,
        mime_type: resource.mime_type,
        file_path: resource.file_path,
        sha256: resource.sha256,
        byte_size: resource.byte_size,
    }
}
