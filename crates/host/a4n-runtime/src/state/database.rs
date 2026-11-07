use a4n_config::app_config::AppConfig;

pub(super) async fn acquire_generation_owner_lock(
    database_url: &str,
) -> Result<a4n_db_session::DbHandle, a4n_error::AppError> {
    a4n_db_session::acquire_generation_owner_lock(database_url).await
}

pub(super) fn validate_runtime_config(config: &AppConfig) -> Result<(), a4n_error::AppError> {
    a4n_config::validate_postgres_database_url("A4N_DATABASE_URL", &config.database_url)?;
    if config
        .security
        .jwt_secret
        .as_deref()
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
        .is_none()
    {
        return Err(a4n_error::AppError::ConfigError(
            "A4N_JWT_SECRET 不能为空".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn preflight_update_migrations(
    database_url: &str,
    data_dir: &std::path::Path,
    names: &[String],
) -> anyhow::Result<a4n_api_contract::updates::UpdateMigrationPreflight> {
    let preflight =
        a4n_migration::migrator::preflight_update_migrations(database_url, data_dir, names).await?;
    Ok(a4n_api_contract::updates::UpdateMigrationPreflight {
        pending_schema_migrations: preflight.pending_schema_migrations,
        pending_data_migrations: preflight.pending_data_migrations,
        migrations: preflight
            .migrations
            .into_iter()
            .map(
                |migration| a4n_api_contract::updates::UpdateDataMigrationPreflight {
                    name: migration.name,
                    affected_projects: migration.affected_projects,
                    media_files_to_copy: migration.media_files_to_copy,
                },
            )
            .collect(),
    })
}

pub(super) fn update_migration_requires_data_backup(name: &str) -> bool {
    a4n_migration::migrator::migration_requires_data_backup(name)
}
