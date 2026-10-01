use super::*;

#[async_trait::async_trait]
impl ThemeReadApplicationPort for ThemeReadWorkerClient {
    async fn list_themes(&self, user_id: i64) -> Result<Vec<ThemeSummary>, AppError> {
        self.command(user_id, THEME_LIST_OPERATION, ThemeUserInput {})
            .await
    }

    async fn export_theme(&self, user_id: i64, theme_id: i64) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_EXPORT_OPERATION, ThemeIdInput { theme_id })
            .await
    }

    async fn list_resources(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<Vec<ThemeResourceRecord>, AppError> {
        self.command(
            user_id,
            THEME_RESOURCES_OPERATION,
            ThemeIdInput { theme_id },
        )
        .await
    }

    async fn get_preference(&self, user_id: i64) -> Result<ThemePreferenceSummary, AppError> {
        self.command(user_id, THEME_PREFERENCE_OPERATION, ThemeUserInput {})
            .await
    }
}
