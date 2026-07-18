use a4n_import_core::{
    ImportNormalizeJsonInput, ImportNormalizeJsonOutput, ImportReadMediaPackageZipInput,
    ImportReadMediaPackageZipOutput, normalize_json, read_media_package_zip,
};
use a4n_service_protocol::{
    HealthStatus, IMPORT_SERVICE_NAME,
    IMPORT_TRANSFORM_CAPABILITY_VERSION as IMPORT_CAPABILITY_VERSION, PROTOCOL_VERSION,
    ServiceError, ServiceErrorCode, ServiceHealth, ServiceRequest, ServiceResponse, ServiceStatus,
    ServiceVersion, redact_sensitive_message, validate_known_service_request_envelope,
};
use axum::{Json, Router, routing::get, routing::post};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_IMPORT_SERVICE_LOG",
        "a4n_import_service=info,tower_http=warn",
    )?;
    let addr = a4n_sidecar_bootstrap::socket_addr("A4N_IMPORT_SERVICE_ADDR", "127.0.0.1:47647")?;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/import/normalize_json", post(normalize_json_route))
        .route(
            "/import/read_media_package_zip",
            post(read_media_package_zip_route),
        );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: IMPORT_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: IMPORT_CAPABILITY_VERSION.to_string(),
        },
    })
}

async fn normalize_json_route(
    Json(request): Json<ServiceRequest<ImportNormalizeJsonInput>>,
) -> Json<ServiceResponse<ImportNormalizeJsonOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let trace_id = request.trace_context.trace_id.clone();
    Json(result_response(trace_id, normalize_json(request.payload)))
}

async fn read_media_package_zip_route(
    Json(request): Json<ServiceRequest<ImportReadMediaPackageZipInput>>,
) -> Json<ServiceResponse<ImportReadMediaPackageZipOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let trace_id = request.trace_context.trace_id.clone();
    Json(result_response(
        trace_id,
        read_media_package_zip(request.payload),
    ))
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
    validate_known_service_request_envelope(request, IMPORT_SERVICE_NAME)
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
        request.capability_version = "import-transform-v0".to_string();

        let error = validate_service_request(&request).expect_err("version mismatch");

        assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
        assert!(error.message.contains("import capability version"));
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
            capability_version: IMPORT_CAPABILITY_VERSION.to_string(),
            payload,
        }
    }
}
