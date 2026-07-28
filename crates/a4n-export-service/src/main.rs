use a4n_export_core::{
    ExportRenderJsonInput, ExportRenderJsonOutput, ExportRenderZipInput, ExportRenderZipOutput,
    render_json, render_zip,
};
use a4n_service_protocol::{
    EXPORT_RENDER_CAPABILITY_VERSION, EXPORT_SERVICE_NAME, EgressDataType, HealthStatus,
    PROTOCOL_VERSION, ServiceError, ServiceErrorCode, ServiceHealth, ServiceRequest,
    ServiceResponse, ServiceStatus, known_service_health_for_host, redact_sensitive_message,
    validate_egress_scope, validate_known_service_request_envelope,
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
    let (shutdown_signal, shutdown) = a4n_sidecar_bootstrap::sidecar_shutdown();
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
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .layer(middleware::from_fn(a4n_sidecar_bootstrap::require_auth))
        .merge(a4n_sidecar_bootstrap::shutdown_router(shutdown_signal));
    let app = a4n_sidecar_bootstrap::with_runtime_metrics(app, EXPORT_SERVICE_NAME)?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
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
) -> Json<ServiceResponse<ExportRenderJsonOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let trace_id = request.trace_context.trace_id.clone();
    Json(result_response(trace_id, render_json(request.payload)))
}

async fn render_zip_route(
    Json(request): Json<ServiceRequest<ExportRenderZipInput>>,
) -> Json<ServiceResponse<ExportRenderZipOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let trace_id = request.trace_context.trace_id.clone();
    Json(result_response(trace_id, render_zip(request.payload)))
}

fn result_response<T, E>(trace_id: String, result: Result<T, E>) -> ServiceResponse<T>
where
    E: std::fmt::Display,
{
    match result {
        Ok(payload) => ServiceResponse {
            status: ServiceStatus::Ok,
            trace_id: Some(trace_id),
            usage: None,
            artifact_refs: Vec::new(),
            payload: Some(payload),
            error: None,
        },
        Err(err) => ServiceResponse {
            status: ServiceStatus::Error,
            trace_id: Some(trace_id),
            usage: None,
            artifact_refs: Vec::new(),
            payload: None,
            error: Some(ServiceError {
                code: ServiceErrorCode::ConfigurationError,
                message: redact_sensitive_message(err.to_string()),
                retryable: Some(false),
            }),
        },
    }
}

fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request_envelope(request, EXPORT_SERVICE_NAME)?;
    validate_egress_scope(
        EXPORT_SERVICE_NAME,
        EgressDataType::Archive,
        &request.secret_policy.scope,
    )
    .map(|_| ())
}

fn error_response<T, P>(request: &ServiceRequest<P>, error: ServiceError) -> ServiceResponse<T> {
    ServiceResponse {
        status: ServiceStatus::Error,
        trace_id: Some(request.trace_context.trace_id.clone()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: None,
        error: Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_incompatible_capability_version() {
        let mut request = service_request(serde_json::json!({}));
        request.capability_version = "export-render-v0".to_string();

        let error = validate_service_request(&request).expect_err("version mismatch");

        assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
        assert!(error.message.contains("export capability version"));
    }

    #[test]
    fn rejects_incompatible_protocol_version_with_trace() {
        let mut request = service_request(serde_json::json!({}));
        request.schema_version.protocol = PROTOCOL_VERSION + 1;

        let response: ServiceResponse<()> =
            error_response(&request, validate_service_request(&request).unwrap_err());

        assert_eq!(response.status, ServiceStatus::Error);
        assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
        let error = response.error.expect("version mismatch error");
        assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
        assert!(error.message.contains("unsupported protocol version"));
    }

    #[test]
    fn rejects_secret_forwarding_policy() {
        let mut request = service_request(serde_json::json!({}));
        request.secret_policy = a4n_service_protocol::SecretForwardingPolicy::local_user_secret();

        let response: ServiceResponse<()> =
            error_response(&request, validate_service_request(&request).unwrap_err());

        assert_eq!(response.status, ServiceStatus::Error);
        assert_eq!(response.trace_id.as_deref(), Some("trace-1"));
        let error = response.error.expect("secret policy error");
        assert_eq!(error.code, ServiceErrorCode::Unauthorized);
        assert!(error.message.contains("不接收 secret forwarding policy"));
    }

    fn service_request<T>(payload: T) -> ServiceRequest<T> {
        ServiceRequest {
            request_id: "request-1".to_string(),
            trace_context: a4n_service_protocol::TraceContext {
                trace_id: "trace-1".to_string(),
                parent_span_id: None,
            },
            schema_version: a4n_service_protocol::SchemaVersion::default(),
            user_id: None,
            project_id: None,
            root_project_id: None,
            runtime_config_snapshot: None,
            secret_policy: a4n_service_protocol::SecretForwardingPolicy::deny_all(),
            capability_version: EXPORT_RENDER_CAPABILITY_VERSION.to_string(),
            payload,
        }
    }
}
