use a4n_service_protocol::{HealthStatus, PROTOCOL_VERSION, ServiceHealth, ServiceVersion};
use axum::{Json, Router, routing::get};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_DEMO_SIDECAR_LOG",
        "a4n_demo_sidecar=info,tower_http=warn",
    )?;
    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_DEMO_SIDECAR_ADDR", "127.0.0.1:47631")?;
    let app = Router::new().route("/healthz", get(healthz));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: "demo".to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: "demo".to_string(),
        },
    })
}
