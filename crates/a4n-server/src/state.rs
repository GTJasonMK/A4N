use a4n_core::config::app_config::{AppConfig, resolve_portable_path};
use a4n_core::error::AppError;
use a4n_core::files::paths::AppPaths;
use a4n_services::auth::BootstrapAdminInput;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

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
        let database_url = required_env("A4N_DATABASE_URL")?;
        validate_postgres_url(&database_url)?;

        let jwt_secret = required_env("A4N_JWT_SECRET")?;
        let bind_addr = std::env::var("A4N_WEB_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .map_err(|e| AppError::ConfigError(format!("A4N_WEB_ADDR 无效: {e}")))?;
        let frontend_dist = std::env::var("A4N_FRONTEND_DIST")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("frontend/dist"));
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
    pub web_events: Arc<WebEventHub>,
}

impl ServerState {
    pub async fn new(config: ServerConfig) -> Result<Self, AppError> {
        let bootstrap_admin = config.bootstrap_admin.clone();
        let app_config = config.into_app_config()?;
        AppPaths::from_config(&app_config)
            .ensure_dirs()
            .map_err(AppError::Io)?;
        let runtime = Arc::new(a4n_runtime::AppState::new(app_config).await?);
        let bootstrap = bootstrap_admin.as_ref().map(|input| BootstrapAdminInput {
            username: input.username.as_str(),
            email: input.email.as_str(),
            password: input.password.as_str(),
        });
        runtime
            .auth
            .require_existing_user_or_bootstrap_admin(bootstrap)
            .await?;

        Ok(Self {
            runtime,
            web_events: Arc::new(WebEventHub::new()),
        })
    }
}

#[derive(Debug, Default)]
pub struct WebEventHub {
    channels: RwLock<std::collections::HashMap<String, broadcast::Sender<serde_json::Value>>>,
}

impl WebEventHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn subscribe(&self, channel: &str) -> broadcast::Receiver<serde_json::Value> {
        let mut channels = self.channels.write().await;
        channels
            .entry(channel.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(1024);
                tx
            })
            .subscribe()
    }

    pub async fn publish(&self, channel: &str, payload: serde_json::Value) -> usize {
        let channels = self.channels.read().await;
        channels
            .get(channel)
            .and_then(|sender| sender.send(payload).ok())
            .unwrap_or(0)
    }
}

impl Deref for ServerState {
    type Target = a4n_runtime::AppState;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
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
        apply_optional_path_env("A4N_CACHE_DIR", |path| config.cache_dir = path)?;
        apply_optional_path_env("A4N_TEMP_DIR", |path| config.temp_dir = path)?;
        apply_optional_path_env("A4N_LOG_DIR", |path| config.log_dir = path)?;
        apply_optional_string_env("A4N_OPENAI_API_KEY", |value| {
            config.llm.api_key = Some(value.clone());
            config.embedding.api_key = Some(value);
        })?;
        apply_optional_string_env("A4N_OPENAI_BASE_URL", |value| {
            config.llm.base_url = value;
        })?;
        Ok(config)
    }
}

fn required_env(key: &str) -> Result<String, AppError> {
    let value = std::env::var(key).map_err(|_| AppError::ConfigError(format!("{key} 不能为空")))?;
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(AppError::ConfigError(format!("{key} 不能为空")));
    }
    Ok(value)
}

fn optional_env(key: &str) -> Result<Option<String>, AppError> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(AppError::ConfigError(format!("{key} 不能为空")))
            } else {
                Ok(Some(value))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(AppError::ConfigError(format!("{key} 无法读取: {err}"))),
    }
}

fn apply_optional_string_env(key: &str, apply: impl FnOnce(String)) -> Result<(), AppError> {
    if let Some(value) = optional_env(key)? {
        apply(value);
    }
    Ok(())
}

fn apply_optional_path_env(key: &str, apply: impl FnOnce(PathBuf)) -> Result<(), AppError> {
    if let Some(value) = optional_env(key)? {
        apply(resolve_portable_path(PathBuf::from(value)));
    }
    Ok(())
}

fn bootstrap_admin_from_env() -> Result<Option<BootstrapAdmin>, AppError> {
    let username = optional_env("A4N_BOOTSTRAP_ADMIN_USERNAME")?;
    let email = optional_env("A4N_BOOTSTRAP_ADMIN_EMAIL")?;
    let password = optional_env("A4N_BOOTSTRAP_ADMIN_PASSWORD")?;

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

fn validate_postgres_url(url: &str) -> Result<(), AppError> {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        Ok(())
    } else {
        Err(AppError::ConfigError(
            "A4N_DATABASE_URL 必须是 postgres:// 或 postgresql:// 连接串".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_postgres_url_accepts_postgres_schemes() {
        validate_postgres_url("postgres://user:password@localhost:5432/a4n").unwrap();
        validate_postgres_url("postgresql://user:password@localhost:5432/a4n").unwrap();
    }

    #[test]
    fn validate_postgres_url_rejects_empty_and_non_postgres_urls() {
        assert!(validate_postgres_url("").is_err());
        assert!(validate_postgres_url("sqlite://data/a4n.db").is_err());
        assert!(validate_postgres_url("mysql://user:password@localhost/a4n").is_err());
    }
}
