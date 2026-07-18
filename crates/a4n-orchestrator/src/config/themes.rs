use a4n_core::{error::AppError, models::user::theme_config};
use serde_json::Value;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn list_themes(&self) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::list_themes(self.db.connection()).await
    }

    pub async fn list_user_themes(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::list_user_themes(self.db.connection(), user_id).await
    }

    pub async fn list_themes_visible_to_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::list_themes_visible_to_user(self.db.connection(), user_id).await
    }

    pub async fn system_default_theme(&self) -> Result<Option<theme_config::Model>, AppError> {
        a4n_db::config::system_default_theme(self.db.connection()).await
    }

    pub async fn theme_by_id(
        &self,
        theme_id: i64,
    ) -> Result<Option<theme_config::Model>, AppError> {
        a4n_db::config::theme_by_id(self.db.connection(), theme_id).await
    }

    pub async fn create_theme(
        &self,
        user_id: Option<i64>,
        name: &str,
        mode: &str,
        css_variables: Value,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::create_theme(self.db.connection(), user_id, name, mode, css_variables).await
    }

    pub async fn update_theme(
        &self,
        theme_id: i64,
        name: Option<&str>,
        mode: Option<&str>,
        css_variables: Option<Value>,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::update_theme(self.db.connection(), theme_id, name, mode, css_variables)
            .await
    }

    pub async fn delete_theme(&self, theme_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_theme(self.db.connection(), theme_id).await
    }

    pub async fn activate_theme(&self, theme_id: i64) -> Result<theme_config::Model, AppError> {
        a4n_db::config::activate_theme(self.db.connection(), theme_id).await
    }
}
