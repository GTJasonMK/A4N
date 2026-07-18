use super::*;

#[test]
fn request_envelope_validation_enforces_request_context_before_handler() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.project_id = None;
    request.root_project_id = Some(7);

    let error = validate_known_service_request_envelope(&request, IMPORT_SERVICE_NAME)
        .expect_err("registered sidecars must reject malformed project scope");
    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(
        error
            .message
            .contains("root_project_id requires project_id")
    );
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn request_envelope_validation_rejects_capability_mismatch() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.capability_version = "import-transform-v0".to_string();

    let error = validate_service_request_envelope(
        &request,
        IMPORT_TRANSFORM_CAPABILITY_VERSION,
        "import capability",
        RequiredSecretPolicy::DenyAll {
            rejection_message: "import transform sidecar 不接收 secret forwarding policy",
        },
    )
    .expect_err("capability mismatch");

    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("import capability version"));
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn request_envelope_validation_rejects_database_schema_for_stateless_sidecars() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.schema_version.database = Some("main-db-v1".to_string());

    let error = validate_service_request_envelope(
        &request,
        IMPORT_TRANSFORM_CAPABILITY_VERSION,
        "import capability",
        RequiredSecretPolicy::DenyAll {
            rejection_message: "import transform sidecar 不接收 secret forwarding policy",
        },
    )
    .expect_err("registered sidecars must reject database schema coupling");

    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert!(error.message.contains("protocol-only"));
    assert!(error.message.contains("main-db-v1"));
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn request_envelope_validation_enforces_deny_all_secret_policy() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.secret_policy = SecretForwardingPolicy::local_user_secret();

    let error = validate_service_request_envelope(
        &request,
        IMPORT_TRANSFORM_CAPABILITY_VERSION,
        "import capability",
        RequiredSecretPolicy::DenyAll {
            rejection_message: "import transform sidecar 不接收 secret forwarding policy",
        },
    )
    .expect_err("deny-all required");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("不接收 secret forwarding policy"));
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn request_envelope_validation_can_defer_secret_policy_to_handler() {
    let mut request = service_request(LLM_CHAT_CAPABILITY_VERSION, json!({}));
    request.secret_policy = SecretForwardingPolicy {
        scope: SecretMaterialScope::TrustedRemote,
        allow_system_secret: false,
        allow_user_secret: true,
        allow_upload_token: true,
    };

    validate_service_request_envelope(
        &request,
        LLM_CHAT_CAPABILITY_VERSION,
        "LLM capability",
        RequiredSecretPolicy::AllowAny,
    )
    .expect("handler-specific secret policy should run after envelope validation");
}

#[test]
fn request_envelope_validation_enforces_local_user_secret_policy() {
    let mut request = service_request(LLM_CHAT_CAPABILITY_VERSION, json!({}));
    request.secret_policy = SecretForwardingPolicy {
        scope: SecretMaterialScope::TrustedRemote,
        allow_system_secret: false,
        allow_user_secret: true,
        allow_upload_token: false,
    };

    let error = validate_service_request_envelope(
        &request,
        LLM_CHAT_CAPABILITY_VERSION,
        "LLM capability",
        RequiredSecretPolicy::LocalUserSecretForwarding {
            rejection_message: "provider API key 只能转发给本地 sidecar",
        },
    )
    .expect_err("local user secret forwarding required");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("本地 sidecar"));
    assert_eq!(error.retryable, Some(false));

    request.secret_policy = SecretForwardingPolicy::local_user_secret();
    validate_service_request_envelope(
        &request,
        LLM_CHAT_CAPABILITY_VERSION,
        "LLM capability",
        RequiredSecretPolicy::LocalUserSecretForwarding {
            rejection_message: "provider API key 只能转发给本地 sidecar",
        },
    )
    .expect("local user secret policy should pass");
}
