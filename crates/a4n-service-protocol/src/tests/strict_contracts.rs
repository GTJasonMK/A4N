use serde::de::DeserializeOwned;

use crate::{CapabilityTaskEvent, ServiceHealth, ServiceRequest, ServiceResponse};

#[test]
fn protocol_envelopes_reject_unknown_fields() {
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
    assert_unknown_field_rejected::<ServiceRequest<serde_json::Value>>(request);

    let response = serde_json::json!({
        "status": "ok",
        "payload": {},
        "future_optional_field": true
    });
    assert_unknown_field_rejected::<ServiceResponse<serde_json::Value>>(response);

    let mut health = serde_json::json!({
        "service": "demo",
        "status": "healthy",
        "version": {
            "service_version": "0.1.0",
            "protocol_version": crate::PROTOCOL_VERSION,
            "capability_version": "demo-v2"
        }
    });
    health["future_optional_field"] = serde_json::json!(true);
    assert_unknown_field_rejected::<ServiceHealth>(health);

    let mut health = serde_json::json!({
        "service": "demo",
        "status": "healthy",
        "version": {
            "service_version": "0.1.0",
            "protocol_version": crate::PROTOCOL_VERSION,
            "capability_version": "demo-v2"
        }
    });
    health["version"]["future_optional_field"] = serde_json::json!(true);
    assert_unknown_field_rejected::<ServiceHealth>(health);

    let event = serde_json::json!({
        "task_id": "task-1",
        "event_kind": "progress",
        "sequence": 1,
        "future_optional_field": true
    });
    assert_unknown_field_rejected::<CapabilityTaskEvent>(event);
}

fn assert_unknown_field_rejected<T: DeserializeOwned>(value: serde_json::Value) {
    let error = serde_json::from_value::<T>(value)
        .err()
        .expect("unknown protocol field must be rejected");
    assert!(
        error
            .to_string()
            .contains("unknown field `future_optional_field`")
    );
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
