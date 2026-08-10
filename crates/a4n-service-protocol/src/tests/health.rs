use super::support::service_health;
use crate::{
    HealthStatus, LLM_SERVICE_NAME, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME,
    PROTOCOL_VERSION, ServiceErrorCode, validate_service_health_envelope,
};

#[test]
fn service_health_validation_rejects_identity_mismatch() {
    let error = validate_service_health_envelope(
        &service_health(
            LLM_SERVICE_NAME,
            HealthStatus::Healthy,
            PROTOCOL_VERSION,
            PROMPT_RENDER_CAPABILITY_VERSION,
        ),
        Some(PROMPT_SERVICE_NAME),
        Some(PROMPT_RENDER_CAPABILITY_VERSION),
    )
    .expect_err("identity mismatch");

    assert_eq!(
        error.service_error_code(),
        ServiceErrorCode::VersionMismatch
    );
    assert!(!error.retryable());
    assert!(error.message().contains("identity mismatch"));
}

#[test]
fn service_health_validation_rejects_unhealthy_status() {
    let error = validate_service_health_envelope(
        &service_health(
            PROMPT_SERVICE_NAME,
            HealthStatus::Degraded,
            PROTOCOL_VERSION,
            PROMPT_RENDER_CAPABILITY_VERSION,
        ),
        Some(PROMPT_SERVICE_NAME),
        Some(PROMPT_RENDER_CAPABILITY_VERSION),
    )
    .expect_err("unhealthy status");

    assert_eq!(
        error.service_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    assert!(error.retryable());
    assert!(error.message().contains("unhealthy"));
}

#[test]
fn service_health_validation_rejects_protocol_mismatch() {
    let error = validate_service_health_envelope(
        &service_health(
            PROMPT_SERVICE_NAME,
            HealthStatus::Healthy,
            PROTOCOL_VERSION + 1,
            PROMPT_RENDER_CAPABILITY_VERSION,
        ),
        Some(PROMPT_SERVICE_NAME),
        Some(PROMPT_RENDER_CAPABILITY_VERSION),
    )
    .expect_err("protocol mismatch");

    assert_eq!(
        error.service_error_code(),
        ServiceErrorCode::VersionMismatch
    );
    assert!(!error.retryable());
    assert!(error.message().contains("protocol version mismatch"));
}

#[test]
fn service_health_validation_rejects_capability_mismatch() {
    let error = validate_service_health_envelope(
        &service_health(
            PROMPT_SERVICE_NAME,
            HealthStatus::Healthy,
            PROTOCOL_VERSION,
            "prompt-render-v0",
        ),
        Some(PROMPT_SERVICE_NAME),
        Some(PROMPT_RENDER_CAPABILITY_VERSION),
    )
    .expect_err("capability mismatch");

    assert_eq!(
        error.service_error_code(),
        ServiceErrorCode::VersionMismatch
    );
    assert!(!error.retryable());
    assert!(error.message().contains("capability version mismatch"));
}

#[test]
fn service_health_validation_allows_generic_healthy_service_without_expectations() {
    validate_service_health_envelope(
        &service_health("demo", HealthStatus::Healthy, PROTOCOL_VERSION, "demo-v1"),
        None,
        None,
    )
    .expect("generic healthy service should pass without explicit expectations");
}

#[test]
fn health_validation_accepts_same_major_rolling_deployments() {
    let health = service_health(
        PROMPT_SERVICE_NAME,
        HealthStatus::Healthy,
        PROTOCOL_VERSION,
        "prompt-render-v4.1",
    );
    validate_service_health_envelope(
        &health,
        Some(PROMPT_SERVICE_NAME),
        Some(PROMPT_RENDER_CAPABILITY_VERSION),
    )
    .expect("same capability major should negotiate");
}

#[test]
fn health_validation_rejects_an_advertised_range_that_excludes_the_host() {
    let mut health = service_health(
        PROMPT_SERVICE_NAME,
        HealthStatus::Healthy,
        PROTOCOL_VERSION,
        PROMPT_RENDER_CAPABILITY_VERSION,
    );
    health.version.compatibility = Some(crate::ServiceCompatibility {
        protocol: crate::VersionRange::exact(PROTOCOL_VERSION + 1),
        capability_major: crate::VersionRange::exact(2),
    });
    let error = validate_service_health_envelope(
        &health,
        Some(PROMPT_SERVICE_NAME),
        Some(PROMPT_RENDER_CAPABILITY_VERSION),
    )
    .expect_err("advertised range must include both host and sidecar");
    assert_eq!(
        error.service_error_code(),
        ServiceErrorCode::VersionMismatch
    );
}
