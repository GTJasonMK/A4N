use super::*;

impl WorkerState {
    pub(super) async fn create_theme(
        &self,
        request: WorkerEnvelope<a4n_api_contract::theme::CreateThemeInput>,
    ) -> Result<ThemeSummary, WorkerFailure> {
        validate_request(&request, THEME_CREATE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let input = request.payload;
        let name = normalize_name(input.name)?;
        a4n_domain::validate_theme_document(&input.document)
            .map_err(WorkerFailure::invalid_request)?;
        self.require_user(user_id).await?;
        self.summary(
            a4n_db::config::create_theme(
                self.database.connection(),
                Some(user_id),
                &name,
                input.mode,
                input.document,
            )
            .await
            .map_err(app_error)?,
        )
        .await
    }

    pub(super) async fn update_theme(
        &self,
        request: WorkerEnvelope<ThemeUpdateInput>,
    ) -> Result<ThemeSummary, WorkerFailure> {
        validate_request(&request, THEME_UPDATE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let theme = self
            .require_owner(user_id, request.payload.theme_id)
            .await?;
        let changes =
            a4n_theme_core::theme_update_changes(request.payload.input, request.payload.resources)
                .map_err(app_error)?;
        let (theme, resources) =
            a4n_db::config::update_theme(self.database.connection(), theme, changes)
                .await
                .map_err(app_error)?;
        theme_summary_with_resources(theme, resources).map_err(app_error)
    }

    pub(super) async fn delete_theme(
        &self,
        request: WorkerEnvelope<ThemeIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, THEME_DELETE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let theme = self
            .require_owner(user_id, request.payload.theme_id)
            .await?;
        if theme.is_system_default {
            return Err(WorkerFailure::invalid_request("系统默认主题不能删除"));
        }
        if theme.is_active {
            return Err(WorkerFailure::invalid_request(
                "正在使用的主题不能删除，请先激活其他主题",
            ));
        }
        a4n_db::config::delete_theme(self.database.connection(), theme.id)
            .await
            .map_err(app_error)
    }

    pub(super) async fn activate_theme(
        &self,
        request: WorkerEnvelope<ThemeIdInput>,
    ) -> Result<ThemeSummary, WorkerFailure> {
        validate_request(&request, THEME_ACTIVATE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        self.require_owner(user_id, request.payload.theme_id)
            .await?;
        self.summary(
            a4n_db::config::activate_theme(self.database.connection(), request.payload.theme_id)
                .await
                .map_err(app_error)?,
        )
        .await
    }

    pub(super) async fn import_theme(
        &self,
        request: WorkerEnvelope<a4n_api_contract::theme::CreateThemeInput>,
    ) -> Result<ThemeSummary, WorkerFailure> {
        validate_request(&request, THEME_IMPORT_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        self.require_user(user_id).await?;
        let mut input = request.payload;
        let base = normalize_name(input.name.clone())?;
        let existing =
            a4n_db::config::list_themes_visible_to_user(self.database.connection(), user_id)
                .await
                .map_err(app_error)?
                .into_iter()
                .filter(|theme| theme.user_id == Some(user_id))
                .map(|theme| theme.name)
                .collect::<std::collections::HashSet<_>>();
        if existing.contains(&base) {
            let mut suffix = 1;
            let mut candidate = format!("{base} (导入)");
            while existing.contains(&candidate) {
                suffix += 1;
                candidate = format!("{base} (导入 {suffix})");
            }
            input.name = candidate;
        } else {
            input.name = base;
        }
        let name = input.name;
        a4n_domain::validate_theme_document(&input.document)
            .map_err(WorkerFailure::invalid_request)?;
        self.summary(
            a4n_db::config::create_theme(
                self.database.connection(),
                Some(user_id),
                &name,
                input.mode,
                input.document,
            )
            .await
            .map_err(app_error)?,
        )
        .await
    }

    pub(super) async fn upsert_resource(
        &self,
        request: WorkerEnvelope<a4n_api_contract::theme::PersistThemeResourceInput>,
    ) -> Result<ThemeResourceRecord, WorkerFailure> {
        validate_request(&request, THEME_RESOURCE_UPSERT_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let input = request.payload;
        self.require_owner(user_id, input.theme_id).await?;
        let resource = a4n_db::config::upsert_theme_resource(
            self.database.connection(),
            input.theme_id,
            &input.resource_key,
            &input.file_name,
            &input.mime_type,
            &input.file_path,
            &input.sha256,
            input.byte_size,
        )
        .await
        .map_err(app_error)?;
        Ok(theme_resource_record(resource))
    }

    pub(super) async fn delete_resource(
        &self,
        request: WorkerEnvelope<ResourceIdInput>,
    ) -> Result<ThemeResourceRecord, WorkerFailure> {
        validate_request(&request, THEME_RESOURCE_DELETE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let resource = a4n_db::config::theme_resource_by_id(
            self.database.connection(),
            request.payload.resource_id,
        )
        .await
        .map_err(app_error)?
        .ok_or_else(|| WorkerFailure::not_found("主题资源不存在"))?;
        self.require_owner(user_id, resource.theme_id).await?;
        let resource =
            a4n_db::config::delete_theme_resource(self.database.connection(), resource.id)
                .await
                .map_err(app_error)?;
        Ok(theme_resource_record(resource))
    }

    pub(super) async fn update_preference(
        &self,
        request: WorkerEnvelope<a4n_api_contract::theme::UpdateThemePreferenceInput>,
    ) -> Result<ThemePreferenceSummary, WorkerFailure> {
        validate_request(&request, THEME_PREFERENCE_UPDATE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        self.require_user(user_id).await?;
        let preference = a4n_db::config::upsert_theme_preference(
            self.database.connection(),
            user_id,
            request.payload.appearance,
        )
        .await
        .map_err(app_error)?;
        theme_preference_summary(user_id, Some(preference)).map_err(app_error)
    }
}
