use crate::routes::{agents::AgentRegistry, page_sessions::PageSessionRegistry};
use a4n_config::app_config::AppConfig;
use a4n_config::{
    optional_non_empty_env_path, optional_non_empty_env_string, required_non_empty_env_string,
    validate_postgres_database_url,
};
use a4n_error::AppError;
use a4n_runtime::RuntimeProfile;
use a4n_runtime::host_api::auth_records;
use a4n_storage::paths::AppPaths;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub agent_worker_token: String,
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
        let agent_worker_token = required_non_empty_env_string("A4N_AGENT_WORKER_TOKEN")?;
        let bind_addr_value = match optional_non_empty_env_string("A4N_CONTROL_ADDR")? {
            Some(value) => value,
            None => optional_non_empty_env_string("A4N_WEB_ADDR")?
                .unwrap_or_else(|| "127.0.0.1:8080".to_string()),
        };
        let bind_addr = bind_addr_value.parse().map_err(|e| {
            AppError::ConfigError(format!("A4N_CONTROL_ADDR/A4N_WEB_ADDR 无效: {e}"))
        })?;
        let frontend_dist = optional_non_empty_env_path("A4N_FRONTEND_DIST")?
            .unwrap_or_else(|| PathBuf::from("frontend/dist"));
        let bootstrap_admin = bootstrap_admin_from_env()?;

        Ok(Self {
            database_url,
            jwt_secret,
            agent_worker_token,
            bind_addr,
            frontend_dist,
            bootstrap_admin,
        })
    }
}

#[derive(Clone)]
pub struct ServerState {
    runtime: Arc<a4n_runtime::AppState>,
    agent_worker_token: Option<String>,
    agents: AgentRegistry,
    page_sessions: PageSessionRegistry,
}

impl ServerState {
    pub async fn new(config: ServerConfig) -> Result<Self, AppError> {
        let bootstrap_admin = config.bootstrap_admin.clone();
        let agent_worker_token = config.agent_worker_token.clone();
        let app_config = config.into_app_config()?;
        AppPaths::new(
            app_config.data_dir.clone(),
            app_config.cache_dir.clone(),
            app_config.temp_dir.clone(),
            app_config.log_dir.clone(),
        )
        .ensure_dirs()
        .map_err(AppError::Io)?;
        let runtime_profile = runtime_profile_for_server_host()?;
        let runtime =
            Arc::new(a4n_runtime::AppState::new_with_profile(app_config, runtime_profile).await?);
        crate::install_agent_host_dispatcher(&runtime)?;
        let bootstrap = bootstrap_admin.as_ref().map(|input| {
            (
                input.username.as_str(),
                input.email.as_str(),
                input.password.as_str(),
            )
        });
        auth_records::require_existing_user_or_bootstrap_admin(runtime.as_ref(), bootstrap).await?;

        Ok(Self {
            runtime,
            agent_worker_token: Some(agent_worker_token),
            agents: AgentRegistry::new(),
            page_sessions: PageSessionRegistry::new(),
        })
    }

    pub fn runtime(&self) -> &a4n_runtime::AppState {
        self.runtime.as_ref()
    }

    pub(crate) fn agents(&self) -> &AgentRegistry {
        &self.agents
    }

    pub(crate) fn page_sessions(&self) -> &PageSessionRegistry {
        &self.page_sessions
    }

    pub(crate) fn agent_worker_token_matches(&self, provided: Option<&str>) -> bool {
        self.agent_worker_token
            .as_deref()
            .zip(provided)
            .is_some_and(|(expected, provided)| expected == provided)
    }

    pub(crate) fn from_runtime(runtime: a4n_runtime::AppState) -> Self {
        let runtime = Arc::new(runtime);
        Self {
            runtime,
            agent_worker_token: None,
            agents: AgentRegistry::new(),
            page_sessions: PageSessionRegistry::new(),
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
        let mut config = a4n_config::app_config_from_env()?;
        config.database_url = self.database_url;
        config.security.jwt_secret = Some(self.jwt_secret);
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
