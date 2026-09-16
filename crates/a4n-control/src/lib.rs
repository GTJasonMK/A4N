mod logging;
mod routes;
mod runtime;
mod state;

#[cfg(test)]
mod media_lifecycle_tests;

use a4n_error::AppError;
use a4n_runtime::AppState;
use axum::{Router, routing::get};
use std::sync::Arc;

pub use logging::init_tracing;
pub use routes::{
    api_router, dispatch_agent_host_command, dispatch_agent_host_command_value,
    dispatch_generation_start, dispatch_runtime_command, dispatch_runtime_command_value,
};
pub use runtime::initialize_runtime;
pub use state::{ServerConfig, ServerState};

/// Installs the Control-owned dispatcher for dynamically discovered Agent tools.
///
/// The same Runtime can be hosted by the Web Control process or by Tauri. Keep
/// this wiring here so both hosts execute the one Control command registry.
pub fn install_agent_host_dispatcher(runtime: &Arc<AppState>) -> Result<(), AppError> {
    let weak_runtime = Arc::downgrade(runtime);
    runtime.set_agent_host_dispatcher(Arc::new(move |user_id, _command, _payload, request| {
        let Some(runtime) = weak_runtime.upgrade() else {
            return Box::pin(async { Err("A4N Host 已退出".to_string()) });
        };
        Box::pin(async move {
            let authorized_user_id = runtime
                .authorize_agent_host_command(&request)
                .await
                .map_err(|error| error.to_string())?;
            if authorized_user_id != user_id {
                return Err("Agent Host command 用户身份与当前 Run 不一致".to_string());
            }
            dispatch_agent_host_command(&runtime, authorized_user_id, request).await
        })
    }))
}

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
        .nest("/api", api_router(state.clone()))
        .nest("/internal", routes::internal_router(state));
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
