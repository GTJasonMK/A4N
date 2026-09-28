//! PostgreSQL-backed chapter-content worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_chapter_content_worker_contract::CHAPTER_CONTENT_WORKER_ID;
use a4n_db::DbHandle;
use a4n_worker_contract::{WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH};
use axum::{
    Router,
    routing::{get, post},
};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

mod auth;
mod commands;
mod errors;
mod health;
mod mutations;
mod permissions;
mod reads;
mod responses;
mod transactions;
mod validation;

const WORKER_ADDR_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47671";

#[derive(Clone)]
pub struct ChapterContentWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

impl ChapterContentWorker {
    pub fn new(
        database: DatabaseConnection,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            state: WorkerState {
                database: DbHandle::new(Arc::new(database)),
                worker_id: Arc::<str>::from(worker_id.into()),
                auth_token: Arc::<str>::from(auth_token.into()),
            },
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route(WORKER_HEALTH_PATH, get(health::health))
            .route(WORKER_CAPABILITIES_PATH, get(health::capabilities))
            .route("/v1/commands/chapter.content.list", post(commands::list))
            .route("/v1/commands/chapter.content.index", post(commands::index))
            .route(
                "/v1/commands/chapter.content.summary",
                post(commands::summary),
            )
            .route(
                "/v1/commands/chapter.content.update",
                post(commands::update),
            )
            .route(
                "/v1/commands/chapter.content.import",
                post(commands::import_chapter),
            )
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    }
}

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    init_tracing();
    let database_url = a4n_config::required_non_empty_env_string(DATABASE_URL_ENV)?;
    a4n_config::validate_postgres_database_url(DATABASE_URL_ENV, &database_url)?;
    let auth_token = a4n_config::required_non_empty_env_string(WORKER_TOKEN_ENV)?;
    let address = worker_addr()?;
    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(8)
        .min_connections(1)
        .connect_timeout(std::time::Duration::from_secs(5));
    let database = Database::connect(options).await?;
    ChapterContentWorker::new(database, CHAPTER_CONTENT_WORKER_ID, auth_token)
        .serve(address)
        .await
}

fn worker_addr() -> anyhow::Result<SocketAddr> {
    let value = env::var(WORKER_ADDR_ENV).unwrap_or_else(|_| DEFAULT_ADDR.into());
    let address: SocketAddr = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{WORKER_ADDR_ENV} 无效: {error}"))?;
    if !address.ip().is_loopback() || address.port() == 0 {
        anyhow::bail!("{WORKER_ADDR_ENV} 必须是 loopback 且端口大于 0")
    }
    Ok(address)
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}
