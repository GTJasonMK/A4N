use a4n_service_protocol::{HealthStatus, PROTOCOL_VERSION, ServiceHealth, ServiceVersion};
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    middleware,
    routing::{get, post},
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_DEMO_SIDECAR_LOG",
        "a4n_demo_sidecar=info,tower_http=warn",
    )?;
    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_DEMO_SIDECAR_ADDR", "127.0.0.1:47631")?;
    let (shutdown_signal, shutdown) = a4n_sidecar_bootstrap::sidecar_shutdown();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/slow", post(slow))
        .route("/counted-slow", post(counted_slow))
        .route("/crash", post(crash))
        .route("/trace", get(trace_headers))
        .layer(middleware::from_fn(a4n_sidecar_bootstrap::require_auth))
        .merge(a4n_sidecar_bootstrap::shutdown_router(shutdown_signal));
    let app = a4n_sidecar_bootstrap::with_runtime_metrics(app, "demo")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn slow() -> StatusCode {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    StatusCode::OK
}

async fn counted_slow() -> StatusCode {
    if let Ok(path) = std::env::var("A4N_DEMO_SIDECAR_COUNTER_FILE") {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("demo counter file should be writable");
        use std::io::Write;
        writeln!(file, "request").expect("demo counter write should succeed");
    }
    slow().await
}

async fn crash() -> StatusCode {
    std::process::exit(17)
}

async fn trace_headers(headers: HeaderMap) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "request_id": headers.get("x-a4n-request-id").and_then(|value| value.to_str().ok()),
        "trace_id": headers.get("x-a4n-trace-id").and_then(|value| value.to_str().ok()),
    }))
}

async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: "demo".to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: "demo".to_string(),
            compatibility: None,
        },
    })
}
