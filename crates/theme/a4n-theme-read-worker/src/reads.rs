use super::*;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ThemeUserInput>,
    ) -> Result<Vec<ThemeSummary>, WorkerFailure> {
        validate_request(&request, THEME_LIST_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let themes =
            a4n_db::config::list_themes_visible_to_user(self.database.connection(), user_id)
                .await
                .map_err(app_error)?;
        let theme_ids = themes.iter().map(|theme| theme.id).collect::<Vec<_>>();
        let resources =
            a4n_db::config::list_theme_resources_for_themes(self.database.connection(), &theme_ids)
                .await
                .map_err(app_error)?;
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
                .map_err(app_error)
            })
            .collect()
    }

    pub(super) async fn export(
        &self,
        request: WorkerEnvelope<ThemeIdInput>,
    ) -> Result<ThemeSummary, WorkerFailure> {
        validate_request(&request, THEME_EXPORT_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let theme = self
            .require_theme_access(user_id, request.payload.theme_id)
            .await?;
        let resources = a4n_db::config::list_theme_resources(self.database.connection(), theme.id)
            .await
            .map_err(app_error)?;
        theme_summary_with_resources(theme, resources).map_err(app_error)
    }

    pub(super) async fn resources(
        &self,
        request: WorkerEnvelope<ThemeIdInput>,
    ) -> Result<Vec<ThemeResourceRecord>, WorkerFailure> {
        validate_request(&request, THEME_RESOURCES_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let theme = self
            .require_theme_access(user_id, request.payload.theme_id)
            .await?;
        a4n_db::config::list_theme_resources(self.database.connection(), theme.id)
            .await
            .map(|resources| resources.into_iter().map(theme_resource_record).collect())
            .map_err(app_error)
    }

    pub(super) async fn preference(
        &self,
        request: WorkerEnvelope<ThemeUserInput>,
    ) -> Result<ThemePreferenceSummary, WorkerFailure> {
        validate_request(&request, THEME_PREFERENCE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let preference = a4n_db::config::theme_preference(self.database.connection(), user_id)
            .await
            .map_err(app_error)?;
        theme_preference_summary(user_id, preference).map_err(app_error)
    }
}
