mod logging;
mod routes;
mod state;

use axum::{Router, routing::get};

pub use logging::init_tracing;
pub use routes::{
    api_router, dispatch_generation_start, dispatch_runtime_command, dispatch_runtime_command_value,
};
pub use state::{ServerConfig, ServerState};

const CONTROL_THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(CONTROL_THREAD_STACK_SIZE)
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    init_tracing()?;

    let config = ServerConfig::from_env()?;
    let bind_addr = config.bind_addr;
    let state = ServerState::new(config).await?;
    let shutdown_state = state.clone();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .nest("/api", api_router(state));
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    tracing::info!("A4N Control listening on http://{bind_addr}");
    let serve_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(anyhow::Error::from);
    let shutdown_result = shutdown_state
        .runtime()
        .shutdown_managed_sidecars()
        .await
        .map_err(anyhow::Error::msg);
    serve_result.and(shutdown_result)
}

async fn healthz() -> &'static str {
    "ok"
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
    tracing::info!("A4N Control shutting down");
}
