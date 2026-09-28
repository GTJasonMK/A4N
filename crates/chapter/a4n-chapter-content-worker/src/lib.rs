//! PostgreSQL-backed chapter-content worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_chapter_content_worker_contract::CHAPTER_CONTENT_WORKER_ID;
use a4n_db::DbHandle;
use a4n_worker_contract::{WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH};
use axum::{
    Router,
    routing::{get, post},
};
use sea_orm::DatabaseConnection;

mod auth;
mod commands;
mod errors;
mod health;
mod mutations;
mod permissions;
mod reads;
mod responses;
mod runtime;
mod transactions;
mod validation;

pub use runtime::run;

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
            .with_graceful_shutdown(runtime::shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    }
}
