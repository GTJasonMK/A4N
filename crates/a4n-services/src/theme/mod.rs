//! 主题配置服务
//!
//! 对应文档 18-主题配置系统

use chrono::Utc;
use sea_orm::*;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::user::theme_config;

#[derive(Clone)]
pub struct ThemeService {
    db: Arc<DatabaseConnection>,
}

impl ThemeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list(&self) -> Result<Vec<theme_config::Model>, AppError> {
        theme_config::Entity::find()
            .order_by_asc(theme_config::Column::Name)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_by_user(&self, user_id: i64) -> Result<Vec<theme_config::Model>, AppError> {
        theme_config::Entity::find()
            .filter(theme_config::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_visible_to_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        theme_config::Entity::find()
            .filter(
                Condition::any()
                    .add(theme_config::Column::UserId.eq(user_id))
                    .add(theme_config::Column::UserId.is_null()),
            )
            .order_by_asc(theme_config::Column::Name)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn system_default(&self) -> Result<Option<theme_config::Model>, AppError> {
        theme_config::Entity::find()
            .filter(theme_config::Column::IsSystemDefault.eq(true))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn create(
        &self,
        user_id: Option<i64>,
        name: &str,
        mode: &str,
        css_variables: serde_json::Value,
    ) -> Result<theme_config::Model, AppError> {
        let now = Utc::now().naive_utc();
        theme_config::ActiveModel {
            user_id: Set(user_id),
            name: Set(name.to_string()),
            mode: Set(mode.to_string()),
            css_variables: Set(css_variables),
            is_system_default: Set(false),
            is_active: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn update(
        &self,
        theme_id: i64,
        name: Option<&str>,
        mode: Option<&str>,
        css_variables: Option<serde_json::Value>,
    ) -> Result<theme_config::Model, AppError> {
        let t = theme_config::Entity::find_by_id(theme_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("主题不存在".into()))?;

        let mut active: theme_config::ActiveModel = t.into();
        if let Some(v) = name {
            active.name = Set(v.to_string());
        }
        if let Some(v) = mode {
            active.mode = Set(v.to_string());
        }
        if let Some(v) = css_variables {
            active.css_variables = Set(v);
        }
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn delete(&self, theme_id: i64) -> Result<(), AppError> {
        theme_config::Entity::delete_by_id(theme_id)
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }
}
