use super::*;

#[test]
fn request_context_builds_request_without_implicit_project_defaults() {
    let mut context = ServiceRequestContext::deny_all(
        "request-1",
        TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: Some("parent-1".to_string()),
        },
    );
    context.user_id = Some(100);
    context.project_id = Some(42);
    context.root_project_id = Some(7);
    context.runtime_config_snapshot = Some(json!({ "profile": "sidecar" }));

    let request = context
        .with_payload(
            IMPORT_TRANSFORM_CAPABILITY_VERSION,
            json!({ "kind": "manifest" }),
        )
        .expect("valid request context should build payload envelope");

    assert_eq!(request.request_id, "request-1");
    assert_eq!(request.trace_context.trace_id, "trace-1");
    assert_eq!(
        request.trace_context.parent_span_id.as_deref(),
        Some("parent-1")
    );
    assert_eq!(request.schema_version.protocol, PROTOCOL_VERSION);
    assert!(request.schema_version.is_protocol_only());
    assert_eq!(request.user_id, Some(100));
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(7));
    assert_eq!(
        request.runtime_config_snapshot,
        Some(json!({ "profile": "sidecar" }))
    );
    assert!(request.secret_policy.is_deny_all());
    assert_eq!(
        request.capability_version,
        IMPORT_TRANSFORM_CAPABILITY_VERSION
    );
    assert_eq!(request.payload["kind"], "manifest");
}

#[test]
fn request_can_be_split_back_into_context_and_payload() {
    let request = ServiceRequestContext::local_user_secret(
        "request-2",
        TraceContext {
            trace_id: "trace-2".to_string(),
            parent_span_id: None,
        },
    )
    .with_payload(
        IMAGE_PROVIDER_CAPABILITY_VERSION,
        json!({ "prompt": "test" }),
    )
    .expect("valid local user secret context should build payload envelope");

    let (context, capability_version, payload) = request.into_context_and_payload();

    assert_eq!(context.request_id, "request-2");
    assert_eq!(context.trace_context.trace_id, "trace-2");
    assert!(context.secret_policy.is_local_user_secret_only());
    assert_eq!(capability_version, IMAGE_PROVIDER_CAPABILITY_VERSION);
    assert_eq!(payload["prompt"], "test");
}

#[test]
fn request_context_with_payload_rejects_invalid_context_at_construction() {
    let mut context = ServiceRequestContext::deny_all(
        "request-1",
        TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
        },
    );
    context.project_id = None;
    context.root_project_id = Some(7);

    let error = context
        .with_payload(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}))
        .expect_err("typed request construction must reject orphan root scope");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(
        error
            .message
            .contains("root_project_id requires project_id")
    );
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn request_context_validation_accepts_project_without_root_scope() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.project_id = Some(42);
    request.root_project_id = None;

    validate_service_request_context(&request)
        .expect("project-only scope is valid for pure transform and RAG requests");
}

#[test]
fn request_context_validation_rejects_blank_request_and_trace_identifiers() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.request_id = "  ".to_string();
    let error = validate_service_request_context(&request).expect_err("blank request id");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("request_id"));
    assert_eq!(error.retryable, Some(false));

    request.request_id = "request-1".to_string();
    request.trace_context.trace_id = " ".to_string();
    let error = validate_service_request_context(&request).expect_err("blank trace id");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("trace_context.trace_id"));

    request.trace_context.trace_id = "trace-1".to_string();
    request.trace_context.parent_span_id = Some(" ".to_string());
    let error = validate_service_request_context(&request).expect_err("blank parent span id");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("parent_span_id"));
}

#[test]
fn request_context_validation_rejects_non_positive_ids_and_orphan_root() {
    for (field, user_id, project_id, root_project_id) in [
        ("user_id", Some(0), None, None),
        ("project_id", None, Some(-1), None),
        ("root_project_id", None, Some(7), Some(0)),
    ] {
        let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
        request.user_id = user_id;
        request.project_id = project_id;
        request.root_project_id = root_project_id;

        let error = validate_service_request_context(&request)
            .expect_err("non-positive request context IDs must be rejected");
        assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
        assert!(error.message.contains(field));
        assert!(error.message.contains("positive integer"));
        assert_eq!(error.retryable, Some(false));
    }

    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.root_project_id = Some(7);
    let error = validate_service_request_context(&request)
        .expect_err("root project scope must not exist without project scope");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(
        error
            .message
            .contains("root_project_id requires project_id")
    );
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn request_context_validation_requires_redacted_object_snapshot() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.runtime_config_snapshot = Some(json!("sidecar"));
    let error = validate_service_request_context(&request)
        .expect_err("runtime config snapshot must be an object");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("must be a JSON object"));
    assert_eq!(error.retryable, Some(false));

    request.runtime_config_snapshot = Some(json!({
        "profile": "sidecar",
        "llm": {
            "api_key_encrypted": "sk-test",
            "base_url": "https://provider.example/v1"
        }
    }));
    let error = validate_service_request_context(&request)
        .expect_err("unredacted runtime config snapshot must be rejected");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("unredacted sensitive fields"));
    assert_eq!(error.retryable, Some(false));

    request.runtime_config_snapshot = Some(json!({
        "profile": "sidecar",
        "llm": {
            "api_key_encrypted": "[REDACTED]",
            "base_url": "[REDACTED]"
        }
    }));
    validate_service_request_context(&request)
        .expect("pre-redacted runtime config snapshot should pass");
}
