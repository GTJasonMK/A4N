use crate::routes::agents::AgentRegistry;
use a4n_core::config::app_config::{AppConfig, apply_storage_env_overrides, portable_home_dir};
use a4n_core::config::env::{
    optional_non_empty_env_path, optional_non_empty_env_string, required_non_empty_env_string,
    validate_postgres_database_url,
};
use a4n_core::error::AppError;
use a4n_core::files::paths::AppPaths;
use a4n_runtime::host_api::auth_records::{self, BootstrapAdminInput};
use a4n_runtime::state::RuntimeProfile;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind_addr: SocketAddr,
    pub frontend_dist: PathBuf,
    pub bootstrap_admin: Option<BootstrapAdmin>,
}

#[derive(Debug, Clone)]
pub struct BootstrapAdmin {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let database_url = required_non_empty_env_string("A4N_DATABASE_URL")?;
        validate_postgres_database_url("A4N_DATABASE_URL", &database_url)?;

        let jwt_secret = required_non_empty_env_string("A4N_JWT_SECRET")?;
        let bind_addr = optional_non_empty_env_string("A4N_WEB_ADDR")?
            .unwrap_or_else(|| "127.0.0.1:8080".to_string())
            .parse()
            .map_err(|e| AppError::ConfigError(format!("A4N_WEB_ADDR 无效: {e}")))?;
        let frontend_dist = optional_non_empty_env_path("A4N_FRONTEND_DIST")?
            .unwrap_or_else(|| PathBuf::from("frontend/dist"));
        let bootstrap_admin = bootstrap_admin_from_env()?;

        Ok(Self {
            database_url,
            jwt_secret,
            bind_addr,
            frontend_dist,
            bootstrap_admin,
        })
    }
}

#[derive(Clone)]
pub struct ServerState {
    runtime: Arc<a4n_runtime::AppState>,
    agents: AgentRegistry,
}

impl ServerState {
    pub async fn new(config: ServerConfig) -> Result<Self, AppError> {
        let bootstrap_admin = config.bootstrap_admin.clone();
        let app_config = config.into_app_config()?;
        AppPaths::from_config(&app_config)
            .ensure_dirs()
            .map_err(AppError::Io)?;
        let runtime_profile = runtime_profile_for_server_host()?;
        let runtime =
            Arc::new(a4n_runtime::AppState::new_with_profile(app_config, runtime_profile).await?);
        let bootstrap = bootstrap_admin.as_ref().map(|input| BootstrapAdminInput {
            username: input.username.as_str(),
            email: input.email.as_str(),
            password: input.password.as_str(),
        });
        auth_records::require_existing_user_or_bootstrap_admin(runtime.as_ref(), bootstrap).await?;

        Ok(Self {
            runtime,
            agents: AgentRegistry::new(),
        })
    }

    pub fn runtime(&self) -> &a4n_runtime::AppState {
        self.runtime.as_ref()
    }

    pub(crate) fn agents(&self) -> &AgentRegistry {
        &self.agents
    }

    pub(crate) fn from_runtime(runtime: a4n_runtime::AppState) -> Self {
        Self {
            runtime: Arc::new(runtime),
            agents: AgentRegistry::new(),
        }
    }
}

fn runtime_profile_for_server_host() -> Result<RuntimeProfile, AppError> {
    let configured = RuntimeProfile::configured_from_env()?;
    #[cfg(not(feature = "embedded-capabilities"))]
    if configured.is_none() {
        tracing::info!(
            runtime_profile = RuntimeProfile::Sidecar.as_str(),
            "server host built without embedded capabilities; defaulting to sidecar profile"
        );
    }
    Ok(configured.unwrap_or_else(RuntimeProfile::default_for_build))
}

impl ServerConfig {
    fn into_app_config(self) -> Result<AppConfig, AppError> {
        let mut config = AppConfig {
            database_url: self.database_url,
            security: a4n_core::config::app_config::SecurityConfig {
                jwt_secret: Some(self.jwt_secret),
            },
            ..Default::default()
        };
        apply_storage_env_overrides(&mut config, &portable_home_dir())?;
        if let Some(value) = optional_non_empty_env_string("A4N_OPENAI_API_KEY")? {
            config.llm.api_key = Some(value.clone());
            config.embedding.api_key = Some(value);
        }
        if let Some(value) = optional_non_empty_env_string("A4N_OPENAI_BASE_URL")? {
            config.llm.base_url = value;
        }
        Ok(config)
    }
}

fn bootstrap_admin_from_env() -> Result<Option<BootstrapAdmin>, AppError> {
    let username = optional_non_empty_env_string("A4N_BOOTSTRAP_ADMIN_USERNAME")?;
    let email = optional_non_empty_env_string("A4N_BOOTSTRAP_ADMIN_EMAIL")?;
    let password = optional_non_empty_env_string("A4N_BOOTSTRAP_ADMIN_PASSWORD")?;

    match (username, email, password) {
        (None, None, None) => Ok(None),
        (Some(username), Some(email), Some(password)) => Ok(Some(BootstrapAdmin {
            username,
            email,
            password,
        })),
        _ => Err(AppError::ConfigError(
            "A4N_BOOTSTRAP_ADMIN_USERNAME、A4N_BOOTSTRAP_ADMIN_EMAIL、A4N_BOOTSTRAP_ADMIN_PASSWORD 必须同时设置"
                .into(),
        )),
    }
}
