mod logging;
mod routes;
mod state;

use axum::Router;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

const SERVER_THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;

fn main() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(SERVER_THREAD_STACK_SIZE)
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    logging::init_tracing()?;

    let config = state::ServerConfig::from_env()?;
    let bind_addr = config.bind_addr;
    let frontend_dist = config.frontend_dist.clone();
    let state = state::ServerState::new(config).await?;
    let app = router(state, frontend_dist);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!("A4N Web server listening on http://{bind_addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    tracing::info!("A4N Web server shutting down");
}

fn router(state: state::ServerState, frontend_dist: PathBuf) -> Router {
    let index_file = frontend_dist.join("index.html");
    let static_files = ServeDir::new(frontend_dist)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index_file));

    Router::new()
        .nest("/api", routes::api_router(state))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
