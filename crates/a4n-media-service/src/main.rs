use std::path::PathBuf;

use a4n_media_contract::{
    MEDIA_SERVICE_ADDR_ENV, MEDIA_SERVICE_DEFAULT_ADDR, MEDIA_SERVICE_NAME, MediaExecuteInput,
    MediaExecuteOutput,
};
use a4n_service_protocol::{
    HealthStatus, SIDECAR_HEALTH_PATH, ServiceError, ServiceErrorCode, ServiceHealth,
    ServiceRequest, ServiceResponse, known_service_health, redact_sensitive_message,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    routing::{get, post},
};

const DATA_DIR_ENV: &str = "A4N_DATA_DIR";
const TEMP_DIR_ENV: &str = "A4N_TEMP_DIR";

#[derive(Clone)]
struct MediaState {
    roots: a4n_media_render_core::MediaRoots,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_MEDIA_SERVICE_LOG",
        "a4n_media_service=info,tower_http=warn",
    )?;
    let addr =
        a4n_sidecar_bootstrap::socket_addr(MEDIA_SERVICE_ADDR_ENV, MEDIA_SERVICE_DEFAULT_ADDR)?;
    let data_dir = std::env::var_os(DATA_DIR_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("{DATA_DIR_ENV} 未配置，媒体 worker 无法启动"))?;
    if !data_dir.is_dir() {
        anyhow::bail!("{DATA_DIR_ENV} 不是目录: {}", data_dir.display());
    }
    let temp_dir = std::env::var_os(TEMP_DIR_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("{TEMP_DIR_ENV} 未配置，媒体 worker 无法启动"))?;
    if !temp_dir.is_dir() {
        anyhow::bail!("{TEMP_DIR_ENV} 不是目录: {}", temp_dir.display());
    }
    let roots = a4n_media_render_core::MediaRoots::new(&data_dir, &temp_dir)
        .map_err(|error| anyhow::anyhow!(error))?;
    let policy = a4n_service_protocol::sidecar_resource_policy(MEDIA_SERVICE_NAME)
        .expect("media service must have a resource policy");
    let app = Router::new()
        .route(SIDECAR_HEALTH_PATH, get(healthz))
        .route(
            "/media/execute",
            post(execute_route).layer(DefaultBodyLimit::max(
                policy.body_limit(a4n_service_protocol::BodyLimitKind::Media),
            )),
        )
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .with_state(MediaState { roots });
    tracing::info!(%addr, "starting media worker");
    a4n_sidecar_bootstrap::serve_sidecar(app, addr, MEDIA_SERVICE_NAME).await
}

async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        MEDIA_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}

async fn execute_route(
    State(state): State<MediaState>,
    Json(request): Json<ServiceRequest<MediaExecuteInput>>,
) -> Json<ServiceResponse<MediaExecuteOutput>> {
    let trace_id = request.trace_context.trace_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        request.execute_known_service(
            MEDIA_SERVICE_NAME,
            |input| a4n_media_render_core::execute(&state.roots, input),
            operation_error,
        )
    })
    .await;
    match result {
        Ok(response) => Json(response),
        Err(error) => Json(ServiceResponse::error(
            trace_id,
            ServiceError::non_retryable(
                ServiceErrorCode::Internal,
                redact_sensitive_message(format!("媒体 worker 执行线程异常终止: {error}")),
            ),
        )),
    }
}

fn operation_error(error: String) -> ServiceError {
    ServiceError::non_retryable(ServiceErrorCode::Internal, redact_sensitive_message(error))
}
