#[async_trait::async_trait]
impl SettingsExportApplicationPort for ConfigReadWorkerClient {
    async fn export_all_configs_to_json(
        &self,
        caller_user_id: i64,
        host: &dyn SettingsAllHostPort,
    ) -> Result<String, AppError> {
        self.command(
            a4n_worker_client::user_principal(caller_user_id)?,
            CONFIG_SETTINGS_EXPORT_OPERATION,
            SettingsExportInput {
                prompts_modified: host.modified_prompts().await?,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl SettingsImportApplicationPort for ConfigReadWorkerClient {
    async fn import_all_configs(
        &self,
        caller_user_id: i64,
        host: &dyn SettingsAllHostPort,
        input: ImportAllConfigsInput,
    ) -> Result<Vec<String>, AppError> {
        let overwrite = input.overwrite;
        let prepared: SettingsPrepareImportResult = self
            .command(
                a4n_worker_client::user_principal(caller_user_id)?,
                CONFIG_SETTINGS_PREPARE_IMPORT_OPERATION,
                input,
            )
            .await?;
        let mut prompt_snapshots = host
            .prepare_prompt_import(&prepared.prompts_modified)
            .await?;
        let imported_names: std::collections::HashSet<_> = match host
            .import_prompts(prepared.prompts_modified.clone(), overwrite)
            .await
        {
            Ok(names) => names.into_iter().collect(),
            Err(error) => {
                return Err(rollback_prompt_import_failure(
                    host,
                    prompt_snapshots,
                    error.to_string(),
                )
                .await);
            }
        };
        let prompt_imported = imported_names.len();
        let prompt_skipped = prepared.prompts_modified.len() - prompt_imported;
        prompt_snapshots.retain(|(name, _)| imported_names.contains(name));
        match self
            .command::<_, Vec<String>>(
                a4n_worker_client::user_principal(caller_user_id)?,
                CONFIG_SETTINGS_IMPORT_OPERATION,
                SettingsImportCommand {
                    settings_json: prepared.settings_json,
                    overwrite,
                    prompt_imported,
                },
            )
            .await
        {
            Ok(mut details) => {
                if prompt_skipped > 0 {
                    details.push(format!("提示词：保留 {prompt_skipped} 个已有自定义"));
                }
                Ok(details)
            }
            Err(error) => {
                Err(rollback_prompt_import_failure(host, prompt_snapshots, error.to_string()).await)
            }
        }
    }
}

async fn rollback_prompt_import_failure(
    host: &dyn SettingsAllHostPort,
    snapshots: Vec<(String, String)>,
    error: String,
) -> AppError {
    if snapshots.is_empty() {
        return AppError::InvalidArgument(error);
    }
    match host.rollback_prompts(snapshots).await {
        Ok(()) => AppError::InvalidArgument(format!("{error}；已回滚本次提示词导入")),
        Err(rollback_error) => {
            AppError::InvalidArgument(format!("{error}；提示词已更新但回滚失败：{rollback_error}"))
        }
    }
}
