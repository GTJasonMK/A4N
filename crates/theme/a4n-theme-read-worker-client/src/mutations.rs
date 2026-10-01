use super::*;

#[async_trait::async_trait]
impl ThemeMutationApplicationPort for ThemeReadWorkerClient {
    async fn create_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_CREATE_OPERATION, input).await
    }

    async fn update_theme(
        &self,
        user_id: i64,
        theme_id: i64,
        input: UpdateThemeInput,
        resources: Vec<PersistThemeResourceInput>,
    ) -> Result<ThemeSummary, AppError> {
        self.command(
            user_id,
            THEME_UPDATE_OPERATION,
            ThemeUpdateInput {
                theme_id,
                input,
                resources,
            },
        )
        .await
    }

    async fn delete_theme(&self, user_id: i64, theme_id: i64) -> Result<(), AppError> {
        let _: serde_json::Value = self
            .command(user_id, THEME_DELETE_OPERATION, ThemeIdInput { theme_id })
            .await?;
        Ok(())
    }

    async fn activate_theme(&self, user_id: i64, theme_id: i64) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_ACTIVATE_OPERATION, ThemeIdInput { theme_id })
            .await
    }

    async fn import_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_IMPORT_OPERATION, input).await
    }

    async fn upsert_resource(
        &self,
        user_id: i64,
        input: PersistThemeResourceInput,
    ) -> Result<ThemeResourceRecord, AppError> {
        self.command(user_id, THEME_RESOURCE_UPSERT_OPERATION, input)
            .await
    }

    async fn delete_resource(
        &self,
        user_id: i64,
        resource_id: i64,
    ) -> Result<ThemeResourceRecord, AppError> {
        self.command(
            user_id,
            THEME_RESOURCE_DELETE_OPERATION,
            ResourceIdInput { resource_id },
        )
        .await
    }

    async fn update_preference(
        &self,
        user_id: i64,
        input: UpdateThemePreferenceInput,
    ) -> Result<ThemePreferenceSummary, AppError> {
        self.command(user_id, THEME_PREFERENCE_UPDATE_OPERATION, input)
            .await
    }
}
