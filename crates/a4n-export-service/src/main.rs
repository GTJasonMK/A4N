use a4n_export_core::{
    ExportRenderJsonInput, ExportRenderZipInput, ExportRenderZipOutput, render_json, render_zip,
};
use a4n_service_protocol::{
    EXPORT_SERVICE_NAME, HealthStatus, ServiceError, ServiceErrorCode, ServiceHealth,
    ServiceRequest, ServiceResponse, known_service_health_for_host, redact_sensitive_message,
};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::HeaderMap,
    middleware,
    routing::{get, post},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_EXPORT_SERVICE_LOG",
        "a4n_export_service=info,tower_http=warn",
    )?;
    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_EXPORT_SERVICE_ADDR", "127.0.0.1:47648")?;
    let policy = a4n_service_protocol::sidecar_resource_policy("export")
        .expect("export sidecar must have a resource policy");
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/export/render_json",
            post(render_json_route).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
            )),
        )
        .route(
            "/export/render_zip",
            post(render_zip_route).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Media),
            )),
        )
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .layer(middleware::from_fn(a4n_sidecar_bootstrap::require_auth));
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, EXPORT_SERVICE_NAME).await
}

async fn healthz(headers: HeaderMap) -> Json<ServiceHealth> {
    Json(known_service_health_for_host(
        EXPORT_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
        a4n_sidecar_bootstrap::host_protocol_version(&headers),
    ))
}

async fn render_json_route(
    Json(request): Json<ServiceRequest<ExportRenderJsonInput>>,
) -> Json<ServiceResponse<String>> {
    Json(request.execute_known_service(EXPORT_SERVICE_NAME, render_json, operation_error))
}

async fn render_zip_route(
    Json(request): Json<ServiceRequest<ExportRenderZipInput>>,
) -> Json<ServiceResponse<ExportRenderZipOutput>> {
    Json(request.execute_known_service(EXPORT_SERVICE_NAME, render_zip, operation_error))
}

fn operation_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::ConfigurationError,
        redact_sensitive_message(error.to_string()),
    )
}
