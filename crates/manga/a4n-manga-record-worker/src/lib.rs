//! PostgreSQL-backed Manga page prompt read worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_db::DbHandle;
use a4n_manga_record_worker_contract::MANGA_RECORD_WORKER_ID;
use axum::Router;

const WORKER_ADDR_ENV: &str = "A4N_MANGA_RECORD_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_MANGA_RECORD_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47676";

#[derive(Clone)]
pub struct MangaRecordWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

mod access;
mod pipeline;
mod prompts;
mod protocol;
mod routes;

impl MangaRecordWorker {
    pub fn new(
        database: DbHandle,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            state: WorkerState {
                database,
                worker_id: Arc::<str>::from(worker_id.into()),
                auth_token: Arc::<str>::from(auth_token.into()),
            },
        }
    }

    pub fn router(&self) -> Router {
        routes::router(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(a4n_worker_runtime::wait_for_ctrl_c())
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
    a4n_worker_runtime::init_worker_tracing();
    let database_url = a4n_config::required_non_empty_env_string(DATABASE_URL_ENV)?;
    a4n_config::validate_postgres_database_url(DATABASE_URL_ENV, &database_url)?;
    let auth_token = a4n_config::required_non_empty_env_string(WORKER_TOKEN_ENV)?;
    let address = a4n_config::worker_listen_addr(WORKER_ADDR_ENV, DEFAULT_ADDR)?;
    let database = a4n_db::manga::connect_manga_record_worker_database(database_url).await?;
    MangaRecordWorker::new(database, MANGA_RECORD_WORKER_ID, auth_token)
        .serve(address)
        .await
}
