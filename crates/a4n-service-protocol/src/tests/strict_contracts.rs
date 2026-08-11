use crate::{CapabilityTaskEvent, ServiceHealth, ServiceRequest, ServiceResponse};

#[test]
fn protocol_envelopes_accept_additive_fields_but_security_contracts_stay_strict() {
    let request = serde_json::json!({
        "request_id": "request-1",
        "trace_context": {"trace_id": "trace-1", "parent_span_id": null},
        "schema_version": {"protocol": crate::PROTOCOL_VERSION},
        "secret_policy": {
            "scope": "none",
            "allow_system_secret": false,
            "allow_user_secret": false,
            "allow_upload_token": false
        },
        "capability_version": "test-v2",
        "payload": {},
        "future_optional_field": true
    });
    serde_json::from_value::<ServiceRequest<serde_json::Value>>(request)
        .expect("old sidecar must ignore additive request envelope fields");

    let response = serde_json::json!({
        "status": "ok",
        "payload": {},
        "future_optional_field": true
    });
    serde_json::from_value::<ServiceResponse<serde_json::Value>>(response)
        .expect("old host must ignore additive response envelope fields");

    let health = serde_json::json!({
        "service": "demo",
        "status": "healthy",
        "future_optional_field": true,
        "version": {
            "service_version": "0.1.0",
            "protocol_version": crate::PROTOCOL_VERSION,
            "capability_version": "demo-v2",
            "compatibility": {
                "protocol": {"min": crate::PROTOCOL_VERSION, "max": crate::PROTOCOL_VERSION},
                "capability_major": {"min": 2, "max": 2}
            },
            "future_optional_field": true
        }
    });
    serde_json::from_value::<ServiceHealth>(health)
        .expect("old host must ignore additive health fields");

    let event = serde_json::json!({
        "task_id": "task-1",
        "event_kind": "progress",
        "sequence": 1,
        "future_optional_field": true
    });
    serde_json::from_value::<CapabilityTaskEvent>(event)
        .expect("old host must ignore additive event envelope fields");

    let strict_secret_policy = serde_json::json!({
        "scope": "none",
        "allow_system_secret": false,
        "allow_user_secret": false,
        "allow_upload_token": false,
        "future_optional_field": true
    });
    let error = serde_json::from_value::<crate::SecretForwardingPolicy>(strict_secret_policy)
        .expect_err("security contracts require an explicit major change");
    assert!(
        error
            .to_string()
            .contains("unknown field `future_optional_field`")
    );
}

#[test]
fn health_accepts_the_version_compatibility_extension() {
    let health = serde_json::json!({
        "service": "demo",
        "status": "healthy",
        "version": {
            "service_version": "0.1.0",
            "protocol_version": crate::PROTOCOL_VERSION,
            "capability_version": "demo-v2",
            "compatibility": {
                "protocol": {"min": crate::PROTOCOL_VERSION, "max": crate::PROTOCOL_VERSION},
                "capability_major": {"min": 2, "max": 2}
            }
        }
    });
    serde_json::from_value::<ServiceHealth>(health)
        .expect("known compatibility extension should remain wire-compatible");
}

#[test]
fn response_distinguishes_explicit_null_payload_from_missing_payload() {
    let unit = serde_json::from_value::<ServiceResponse<()>>(serde_json::json!({
        "status": "ok",
        "payload": null
    }))
    .expect("unit payload should deserialize from explicit null");
    assert_eq!(unit.payload, Some(()));

    let missing = serde_json::from_value::<ServiceResponse<()>>(serde_json::json!({
        "status": "ok"
    }))
    .expect("missing payload remains representable for contract validation");
    assert_eq!(missing.payload, None);
}
