use a4n_config::app_config::AppConfig;

#[cfg(feature = "migration")]
pub(super) async fn run_startup_migrations(
    db: &a4n_db_session::DbHandle,
) -> Result<(), a4n_error::AppError> {
    db.run_migrations().await
}

#[cfg(not(feature = "migration"))]
pub(super) async fn run_startup_migrations(
    db: &a4n_db_session::DbHandle,
) -> Result<(), a4n_error::AppError> {
    db.verify_required_schema().await
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
