use a4n_config_read_core::settings::{
    AllConfigsExport, SETTINGS_ALL_FORMAT_VERSION, import_details_from_counts,
    normalize_all_configs_export, validate_all_configs_export,
};
use a4n_config_read_worker_contract::{
    CONFIG_SETTINGS_EXPORT_OPERATION, CONFIG_SETTINGS_IMPORT_OPERATION,
    CONFIG_SETTINGS_PREPARE_IMPORT_OPERATION,
};
use a4n_db::config::{SettingsAllConfigImportInput, SettingsAllConfigRows};
use chrono::Utc;

impl WorkerState {
    async fn settings_export(
        &self,
        request: WorkerEnvelope<SettingsExportInput>,
    ) -> Result<String, WorkerFailure> {
        validate_admin_request(&request, CONFIG_SETTINGS_EXPORT_OPERATION, &self.database).await?;
        let rows = a4n_db::config::settings_all_config_rows(self.database.connection())
            .await
            .map_err(app_error)?;
        serde_json::to_string_pretty(&AllConfigsExport {
            version: SETTINGS_ALL_FORMAT_VERSION.to_string(),
            export_time: Utc::now().to_rfc3339(),
            llm_configs: rows.llm_configs,
            embedding_configs: rows.embedding_configs,
            image_configs: rows.image_configs,
            video_configs: rows.video_configs,
            prompts_modified: request.payload.prompts_modified,
            themes: rows.themes,
            system_settings: rows.system_settings,
        })
        .map_err(|error| WorkerFailure::unavailable(format!("配置导出序列化失败: {error}")))
    }

    async fn settings_prepare_import(
        &self,
        request: WorkerEnvelope<a4n_api_contract::config::ImportAllConfigsInput>,
    ) -> Result<SettingsPrepareImportResult, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_SETTINGS_PREPARE_IMPORT_OPERATION,
            &self.database,
        )
        .await?;
        let mut incoming = parse_settings_export(&request.payload.settings_json)?;
        normalize_all_configs_export(&mut incoming).map_err(WorkerFailure::invalid_request)?;
        validate_all_configs_export(&incoming).map_err(WorkerFailure::invalid_request)?;
        let config_import = settings_import_input(&incoming, request.payload.overwrite);
        a4n_db::config::validate_settings_all_config_import_defaults(
            self.database.connection(),
            &config_import,
        )
        .await
        .map_err(settings_import_error)?;
        let settings_json = serde_json::to_string(&incoming)
            .map_err(|error| WorkerFailure::unavailable(format!("配置导入序列化失败: {error}")))?;
        Ok(SettingsPrepareImportResult {
            settings_json,
            prompts_modified: incoming.prompts_modified,
        })
    }

    async fn settings_import(
        &self,
        request: WorkerEnvelope<SettingsImportCommand>,
    ) -> Result<Vec<String>, WorkerFailure> {
        validate_admin_request(&request, CONFIG_SETTINGS_IMPORT_OPERATION, &self.database).await?;
        let mut incoming = parse_settings_export(&request.payload.settings_json)?;
        normalize_all_configs_export(&mut incoming).map_err(WorkerFailure::invalid_request)?;
        validate_all_configs_export(&incoming).map_err(WorkerFailure::invalid_request)?;
        if request.payload.prompt_imported > incoming.prompts_modified.len() {
            return Err(WorkerFailure::invalid_request(
                "导入提示词数量不能超过导出内容中的提示词数量",
            ));
        }
        let config_import = settings_import_input(&incoming, request.payload.overwrite);
        a4n_db::config::validate_settings_all_config_import_defaults(
            self.database.connection(),
            &config_import,
        )
        .await
        .map_err(settings_import_error)?;
        let counts =
            a4n_db::config::import_settings_all_configs(self.database.connection(), config_import)
                .await
                .map_err(settings_import_error)?;
        Ok(import_details_from_counts(
            counts,
            request.payload.prompt_imported,
        ))
    }
}

fn parse_settings_export(settings_json: &str) -> Result<AllConfigsExport, WorkerFailure> {
    serde_json::from_str(settings_json)
        .map_err(|error| WorkerFailure::invalid_request(format!("JSON 解析失败：{error}")))
}

fn settings_import_input(
    incoming: &AllConfigsExport,
    overwrite: bool,
) -> SettingsAllConfigImportInput {
    SettingsAllConfigImportInput {
        overwrite,
        rows: SettingsAllConfigRows {
            llm_configs: incoming.llm_configs.clone(),
            embedding_configs: incoming.embedding_configs.clone(),
            image_configs: incoming.image_configs.clone(),
            video_configs: incoming.video_configs.clone(),
            themes: incoming.themes.clone(),
            system_settings: incoming.system_settings.clone(),
        },
    }
}

fn settings_import_error(error: a4n_db::config::SettingsAllConfigImportError) -> WorkerFailure {
    match error {
        a4n_db::config::SettingsAllConfigImportError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::config::SettingsAllConfigImportError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}
