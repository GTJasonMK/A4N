use a4n_core::{error::AppError, models::user::theme_config};
use serde_json::Value;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn create_theme(
        &self,
        user_id: Option<i64>,
        name: &str,
        mode: &str,
        css_variables: Value,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::create_theme(
            self.database.connection(),
            user_id,
            name,
            mode,
            css_variables,
        )
        .await
    }

    pub async fn update_theme(
        &self,
        theme_id: i64,
        name: Option<&str>,
        mode: Option<&str>,
        css_variables: Option<Value>,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::update_theme(
            self.database.connection(),
            theme_id,
            name,
            mode,
            css_variables,
        )
        .await
    }

    pub async fn delete_theme(&self, theme_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_theme(self.database.connection(), theme_id).await
    }
}
