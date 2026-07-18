use super::contracts::*;
use super::embedding::*;
use super::llm::*;
use super::media::*;
use super::package_io::*;
use super::prompt::*;
use super::rag::*;
use super::transport::*;
use a4n_export_core::ExportRenderZipOutput;
use a4n_import_core::{ImportNormalizeJsonOutput, ImportReadMediaPackageZipInput};
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, EXPORT_SERVICE_NAME, HealthStatus, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, LLM_SERVICE_NAME, PROMPT_SERVICE_NAME, PROTOCOL_VERSION, RAG_SERVICE_NAME,
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse, VIDEO_SERVICE_NAME,
    expected_capability_version_for_service,
};
use std::sync::Arc;

#[test]
fn sidecar_transport_joins_paths_without_double_slashes() {
    let transport = HttpSidecarTransport::new("http://127.0.0.1:47641/");

    assert_eq!(transport.url("/healthz"), "http://127.0.0.1:47641/healthz");
}

#[test]
fn health_validation_rejects_protocol_mismatch() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &health(
            PROMPT_SERVICE_NAME,
            PROTOCOL_VERSION + 1,
            prompt_capability_version,
        ),
        Some(PROMPT_SERVICE_NAME),
        Some(prompt_capability_version),
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::VersionMismatch);
    assert!(err.message.contains("protocol version mismatch"));
    assert_eq!(err.retryable, Some(false));
}

#[test]
fn health_validation_rejects_expected_capability_mismatch() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &health(PROMPT_SERVICE_NAME, PROTOCOL_VERSION, "prompt-render-v0"),
        Some(PROMPT_SERVICE_NAME),
        Some(prompt_capability_version),
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::VersionMismatch);
    assert!(err.message.contains("capability version mismatch"));
    assert_eq!(err.retryable, Some(false));
}

#[test]
fn health_validation_rejects_expected_service_identity_mismatch() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &health(
            LLM_SERVICE_NAME,
            PROTOCOL_VERSION,
            prompt_capability_version,
        ),
        Some(PROMPT_SERVICE_NAME),
        Some(prompt_capability_version),
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::VersionMismatch);
    assert!(err.message.contains("service identity mismatch"));
    assert_eq!(err.retryable, Some(false));
}

#[test]
fn health_validation_rejects_unhealthy_status() {
    let prompt_capability_version = capability_version_for(PROMPT_SERVICE_NAME);
    let err = validate_service_health(
        &a4n_service_protocol::ServiceHealth {
            service: PROMPT_SERVICE_NAME.to_string(),
            status: HealthStatus::Unavailable,
            version: a4n_service_protocol::ServiceVersion {
                service_version: "test".to_string(),
                protocol_version: PROTOCOL_VERSION,
                capability_version: prompt_capability_version.to_string(),
            },
        },
        Some(PROMPT_SERVICE_NAME),
        Some(prompt_capability_version),
    )
    .unwrap_err();

    assert_eq!(err.code, ServiceErrorCode::ServiceUnavailable);
    assert!(err.message.contains("unhealthy"));
    assert_eq!(err.retryable, Some(true));
}

#[test]
fn health_validation_allows_unknown_capability_when_no_expectation_is_set() {
    validate_service_health(&health("demo", PROTOCOL_VERSION, "demo-v1"), None, None)
        .expect("generic transport should not reject unknown capabilities");
}

#[test]
fn typed_transport_rejects_unregistered_service_contract_without_panicking() {
    let error = validate_health_expectation(Some("unknown"), None)
        .expect_err("typed transport must require a registered capability version");

    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("unknown"));
    assert!(error.message.contains("未注册 capability version"));
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn typed_http_clients_set_expected_service_names_and_capability_versions() {
    assert_eq!(
        HttpPromptClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(PROMPT_SERVICE_NAME)
    );
    assert_eq!(
        HttpPromptClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(PROMPT_SERVICE_NAME))
    );
    assert_eq!(
        HttpLlmClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(LLM_SERVICE_NAME)
    );
    assert_eq!(
        HttpLlmClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(LLM_SERVICE_NAME))
    );
    assert_eq!(
        HttpEmbeddingClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(EMBEDDING_SERVICE_NAME)
    );
    assert_eq!(
        HttpEmbeddingClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(EMBEDDING_SERVICE_NAME))
    );
    assert_eq!(
        HttpRagClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(RAG_SERVICE_NAME)
    );
    assert_eq!(
        HttpRagClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(RAG_SERVICE_NAME))
    );
    assert_eq!(
        HttpImageClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(IMAGE_SERVICE_NAME)
    );
    assert_eq!(
        HttpImageClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(IMAGE_SERVICE_NAME))
    );
    assert_eq!(
        HttpVideoClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(VIDEO_SERVICE_NAME)
    );
    assert_eq!(
        HttpVideoClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(VIDEO_SERVICE_NAME))
    );
    assert_eq!(
        HttpImportClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(IMPORT_SERVICE_NAME)
    );
    assert_eq!(
        HttpImportClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(IMPORT_SERVICE_NAME))
    );
    assert_eq!(
        HttpExportClient::new("http://127.0.0.1:1")
            .transport
            .expected_service_name,
        Some(EXPORT_SERVICE_NAME)
    );
    assert_eq!(
        HttpExportClient::new("http://127.0.0.1:1")
            .transport
            .expected_capability_version,
        Some(capability_version_for(EXPORT_SERVICE_NAME))
    );
}

#[test]
fn import_export_typed_clients_are_trait_object_safe() {
    let import: Arc<dyn ImportClient> = Arc::new(HttpImportClient::new("http://127.0.0.1:1"));
    let export: Arc<dyn ExportClient> = Arc::new(HttpExportClient::new("http://127.0.0.1:1"));

    assert_import_client_object(import);
    assert_export_client_object(export);
}

#[test]
fn typed_import_request_conversion_preserves_context_and_payload() {
    let request = typed_test_request(
        capability_version_for(IMPORT_SERVICE_NAME),
        ImportReadMediaPackageZipInput {
            zip_bytes_base64: "UEsDBAo=".to_string(),
            manifest_path: "manifest.json".to_string(),
            package_label: "项目包".to_string(),
            manifest_label: "项目包清单".to_string(),
        },
    );

    let json_request = into_json_request(request).expect("typed request should serialize");

    assert_eq!(json_request.request_id, "request-1");
    assert_eq!(json_request.trace_context.trace_id, "trace-1");
    assert_eq!(json_request.project_id, Some(42));
    assert_eq!(json_request.root_project_id, Some(7));
    assert_eq!(
        json_request.capability_version,
        capability_version_for(IMPORT_SERVICE_NAME)
    );
    assert!(json_request.secret_policy.is_deny_all());
    assert_eq!(json_request.payload["manifest_path"], "manifest.json");
    assert_eq!(json_request.payload["package_label"], "项目包");
}

#[test]
fn typed_request_conversion_rejects_invalid_context_before_transport() {
    let mut request = typed_test_request(
        capability_version_for(IMPORT_SERVICE_NAME),
        ImportReadMediaPackageZipInput {
            zip_bytes_base64: "UEsDBAo=".to_string(),
            manifest_path: "manifest.json".to_string(),
            package_label: "项目包".to_string(),
            manifest_label: "项目包清单".to_string(),
        },
    );
    request.runtime_config_snapshot = Some(serde_json::json!({
        "api_key": "sk-unredacted"
    }));

    let error = into_json_request(request)
        .expect_err("typed request conversion must reject unredacted context");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("unredacted sensitive fields"));
    assert_eq!(error.retryable, Some(false));
    assert!(!error.message.contains("sk-unredacted"));
}

#[test]
fn typed_export_response_conversion_deserializes_payload() {
    let response = ServiceResponse {
        status: a4n_service_protocol::ServiceStatus::Ok,
        trace_id: Some("trace-1".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(serde_json::json!({
            "bytes_base64": "UEsDBAo=",
            "content_type": "application/zip"
        })),
        error: None,
    };

    let typed: ServiceResponse<ExportRenderZipOutput> =
        from_json_response(response).expect("typed response should deserialize");

    let payload = typed.payload.expect("typed payload");
    assert_eq!(payload.bytes_base64, "UEsDBAo=");
    assert_eq!(payload.content_type, "application/zip");
}

#[test]
fn typed_response_conversion_preserves_error_without_payload_deserialization() {
    let response = ServiceResponse {
        status: a4n_service_protocol::ServiceStatus::Error,
        trace_id: Some("trace-1".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: None,
        error: Some(ServiceError {
            code: ServiceErrorCode::ConfigurationError,
            message: "bad package".to_string(),
            retryable: Some(false),
        }),
    };

    let typed: ServiceResponse<ImportNormalizeJsonOutput> =
        from_json_response(response).expect("error response should not require payload");

    assert!(typed.payload.is_none());
    assert_eq!(
        typed.error.expect("service error").code,
        ServiceErrorCode::ConfigurationError
    );
}

#[test]
fn embedding_model_operation_paths_are_explicit() {
    assert_eq!(
        embedding_model_operation_path("list").expect("list path"),
        "/embedding/models"
    );
    assert_eq!(
        embedding_model_operation_path("download_info").expect("download info path"),
        "/embedding/model/download_info"
    );
    assert_eq!(
        embedding_model_operation_path("download").expect("download path"),
        "/embedding/model/download"
    );
    assert_eq!(
        embedding_model_operation_path("delete").expect("delete path"),
        "/embedding/model/delete"
    );
    assert_eq!(
        EMBEDDING_MODEL_DOWNLOAD_EVENTS_PATH,
        "/embedding/model/download/events"
    );
}

#[test]
fn embedding_model_operation_path_rejects_unknown_operation() {
    for operation in ["models", "download_events"] {
        let err = embedding_model_operation_path(operation).unwrap_err();

        assert_eq!(err.code, ServiceErrorCode::ConfigurationError);
        assert!(
            err.message
                .contains("unsupported embedding model operation")
        );
        assert_eq!(err.retryable, Some(false));
    }
}

#[test]
fn drains_ndjson_events_across_chunks() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut buffer =
        br#"{"task_id":"task","event_kind":"chunk","sequence":1,"chunk_delta":"he"}"#.to_vec();

    let terminal = drain_event_lines(&mut buffer, &tx).expect("partial line should stay buffered");
    assert!(!terminal);
    assert!(rx.try_recv().is_err());

    buffer.extend_from_slice(
        br#"
{"task_id":"task","event_kind":"completed","sequence":2}
"#,
    );
    let terminal = drain_event_lines(&mut buffer, &tx).expect("complete lines should drain");
    assert!(terminal);

    let first = rx.try_recv().expect("chunk event");
    assert_eq!(
        first.event_kind,
        a4n_service_protocol::ServiceTaskEventKind::Chunk
    );
    assert_eq!(first.chunk_delta.as_deref(), Some("he"));

    let second = rx.try_recv().expect("completed event");
    assert_eq!(
        second.event_kind,
        a4n_service_protocol::ServiceTaskEventKind::Completed
    );
}

#[test]
fn drain_event_lines_stops_after_terminal_event() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut buffer = br#"{"task_id":"task","event_kind":"completed","sequence":1}
{"task_id":"task","event_kind":"chunk","sequence":2,"chunk_delta":"late"}
"#
    .to_vec();

    let terminal = drain_event_lines(&mut buffer, &tx).expect("terminal line should drain");
    assert!(terminal);
    assert!(buffer.is_empty());

    let event = rx.try_recv().expect("completed event");
    assert_eq!(
        event.event_kind,
        a4n_service_protocol::ServiceTaskEventKind::Completed
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn parse_event_line_rejects_remote_local_artifact_path() {
    let err = parse_event_line(
        br#"{"task_id":"task","event_kind":"artifact","sequence":1,"artifact_refs":[{"artifact_id":"artifact-1","artifact_kind":"package","content_type":"application/zip","sha256":"abc","byte_size":10,"location":{"local_path":"/tmp/package.zip"},"ownership":"remote"}]}"#,
    )
    .unwrap_err();

    assert_eq!(err.code, a4n_service_protocol::ServiceErrorCode::Internal);
    assert!(err.message.contains("artifact ref 非法"));
    assert!(err.message.contains("remote sidecar artifact"));
}

#[test]
fn parse_event_line_redacts_error_message() {
    let event = parse_event_line(
        br#"{"task_id":"task","event_kind":"failed","sequence":1,"error":{"code":"provider_error","message":"provider failed: base_url=https://provider.example api_key=sk-test","retryable":false}}"#,
    )
    .expect("event should parse");

    let message = event.error.expect("error").message;
    assert!(message.contains("base_url=[REDACTED]"));
    assert!(message.contains("api_key=[REDACTED]"));
    assert!(!message.contains("provider.example"));
    assert!(!message.contains("sk-test"));
}

fn typed_test_request<T>(capability_version: &str, payload: T) -> ServiceRequest<T> {
    let mut context = a4n_service_protocol::ServiceRequestContext::deny_all(
        "request-1",
        a4n_service_protocol::TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: Some("parent-1".to_string()),
        },
    );
    context.user_id = Some(100);
    context.project_id = Some(42);
    context.root_project_id = Some(7);
    context.runtime_config_snapshot = Some(serde_json::json!({ "profile": "sidecar" }));
    context
        .with_payload(capability_version, payload)
        .expect("typed test request context should be valid")
}

fn assert_import_client_object(_: Arc<dyn ImportClient>) {}

fn assert_export_client_object(_: Arc<dyn ExportClient>) {}

fn capability_version_for(service_name: &'static str) -> &'static str {
    expected_capability_version_for_service(service_name)
        .expect("test service must have protocol capability version")
}

fn health(
    service: &str,
    protocol_version: u32,
    capability_version: &str,
) -> a4n_service_protocol::ServiceHealth {
    a4n_service_protocol::ServiceHealth {
        service: service.to_string(),
        status: a4n_service_protocol::HealthStatus::Healthy,
        version: a4n_service_protocol::ServiceVersion {
            service_version: "test".to_string(),
            protocol_version,
            capability_version: capability_version.to_string(),
        },
    }
}
