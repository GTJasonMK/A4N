use a4n_import_contract::{
    IMPORT_SERVICE_NAME, ImportReadMediaPackageZipInput, ImportReadMediaPackageZipOutput,
};
use a4n_import_core::read_media_package_zip;
use a4n_service_protocol::{
    HealthStatus, SIDECAR_HEALTH_PATH, ServiceError, ServiceErrorCode, ServiceHealth,
    ServiceRequest, ServiceResponse, known_service_health, redact_sensitive_message,
};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_IMPORT_SERVICE_LOG",
        "a4n_import_service=info,tower_http=warn",
    )?;
    let addr = a4n_sidecar_bootstrap::service_socket_addr(IMPORT_SERVICE_NAME)?;
    let policy = a4n_service_protocol::sidecar_resource_policy(IMPORT_SERVICE_NAME)
        .expect("import sidecar must have a resource policy");
    let app = Router::new()
        .route(SIDECAR_HEALTH_PATH, get(healthz))
        .route(
            "/import/read_media_package_zip",
            post(read_media_package_zip_route).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Media),
            )),
        )
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ));
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, IMPORT_SERVICE_NAME).await
}

async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        IMPORT_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}

async fn read_media_package_zip_route(
    Json(request): Json<ServiceRequest<ImportReadMediaPackageZipInput>>,
) -> Json<ServiceResponse<ImportReadMediaPackageZipOutput>> {
    Json(request.execute_known_service(
        IMPORT_SERVICE_NAME,
        read_media_package_zip,
        operation_error,
    ))
}

fn operation_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::ConfigurationError,
        redact_sensitive_message(error.to_string()),
    )
}
