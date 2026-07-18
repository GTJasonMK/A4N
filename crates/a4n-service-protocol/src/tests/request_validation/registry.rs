use super::*;

#[test]
fn known_service_request_envelope_validation_uses_service_registry() {
    let mut request = service_request(IMPORT_TRANSFORM_CAPABILITY_VERSION, json!({}));
    request.secret_policy = SecretForwardingPolicy::local_user_secret();

    let error = validate_known_service_request_envelope(&request, IMPORT_SERVICE_NAME)
        .expect_err("import sidecar should require deny-all secret policy");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(
        error
            .message
            .contains("import transform sidecar 不接收 secret forwarding policy")
    );

    request.secret_policy = SecretForwardingPolicy::deny_all();
    validate_known_service_request_envelope(&request, IMPORT_SERVICE_NAME)
        .expect("registered import envelope should pass");
}

#[test]
fn known_provider_service_request_envelope_defers_secret_policy_to_handler() {
    let mut request = service_request(IMAGE_PROVIDER_CAPABILITY_VERSION, json!({}));
    request.secret_policy = SecretForwardingPolicy {
        scope: SecretMaterialScope::TrustedRemote,
        allow_system_secret: false,
        allow_user_secret: true,
        allow_upload_token: true,
    };

    validate_known_service_request_envelope(&request, IMAGE_SERVICE_NAME)
        .expect("provider sidecar envelope should defer API key policy to handler");
}

#[test]
fn provider_api_key_secret_policy_allows_empty_key_without_user_secret() {
    validate_provider_api_key_secret_policy(
        None,
        &SecretForwardingPolicy::deny_all(),
        LLM_SERVICE_NAME,
    )
    .expect("missing API key should not require user secret forwarding");
    validate_provider_api_key_secret_policy(
        Some("  "),
        &SecretForwardingPolicy::deny_all(),
        IMAGE_SERVICE_NAME,
    )
    .expect("blank API key should not require user secret forwarding");
}

#[test]
fn provider_api_key_secret_policy_rejects_trusted_remote() {
    let policy = SecretForwardingPolicy {
        scope: SecretMaterialScope::TrustedRemote,
        allow_system_secret: false,
        allow_user_secret: true,
        allow_upload_token: false,
    };

    let error =
        validate_provider_api_key_secret_policy(Some("sk-test"), &policy, VIDEO_SERVICE_NAME)
            .expect_err("remote sidecar must not receive provider API key");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("video provider api_key"));
    assert!(error.message.contains("local_sidecar_only"));
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn provider_api_key_secret_policy_rejects_extra_secret_permissions() {
    for policy in [
        SecretForwardingPolicy {
            allow_upload_token: true,
            ..SecretForwardingPolicy::local_user_secret()
        },
        SecretForwardingPolicy {
            allow_system_secret: true,
            ..SecretForwardingPolicy::local_user_secret()
        },
    ] {
        let error =
            validate_provider_api_key_secret_policy(Some("sk-test"), &policy, LLM_SERVICE_NAME)
                .expect_err("provider API key policy must not grant unrelated secret access");

        assert_eq!(error.code, ServiceErrorCode::Unauthorized);
        assert!(error.message.contains("user-secret-only"));
        assert_eq!(error.retryable, Some(false));
    }
}

#[test]
fn provider_api_key_secret_policy_rejects_unknown_service() {
    let error = validate_provider_api_key_secret_policy(
        Some("sk-test"),
        &SecretForwardingPolicy::deny_all(),
        "demo",
    )
    .expect_err("unknown provider sidecar service should fail explicitly");

    assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
    assert!(error.message.contains("unknown provider sidecar service"));
    assert_eq!(error.retryable, Some(false));
}

#[test]
fn known_service_health_uses_service_registry() {
    let health = known_service_health(PROMPT_SERVICE_NAME, "1.2.3", HealthStatus::Healthy);

    assert_eq!(health.service, PROMPT_SERVICE_NAME);
    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.version.service_version, "1.2.3");
    assert_eq!(health.version.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        health.version.capability_version,
        PROMPT_RENDER_CAPABILITY_VERSION
    );
}
