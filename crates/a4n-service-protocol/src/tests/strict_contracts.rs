use crate::{ServiceHealth, ServiceRequest, ServiceResponse, ServiceTaskEvent};

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
        "legacy_field": true
    });
    let error = serde_json::from_value::<ServiceRequest<serde_json::Value>>(request)
        .expect_err("unknown request envelope fields must fail");
    assert!(error.to_string().contains("unknown field `legacy_field`"));

    let response = serde_json::json!({
        "status": "ok",
        "payload": {},
        "legacy_field": true
    });
    let error = serde_json::from_value::<ServiceResponse<serde_json::Value>>(response)
        .expect_err("unknown response envelope fields must fail");
    assert!(error.to_string().contains("unknown field `legacy_field`"));

    let health = serde_json::json!({
        "service": "demo",
        "status": "healthy",
        "version": {
            "service_version": "0.1.0",
            "protocol_version": crate::PROTOCOL_VERSION,
            "capability_version": "demo-v2",
            "legacy_field": true
        }
    });
    let error = serde_json::from_value::<ServiceHealth>(health)
        .expect_err("unknown health envelope fields must fail");
    assert!(error.to_string().contains("unknown field `legacy_field`"));

    let event = serde_json::json!({
        "task_id": "task-1",
        "event_kind": "progress",
        "sequence": 1,
        "legacy_field": true
    });
    let error = serde_json::from_value::<ServiceTaskEvent>(event)
        .expect_err("unknown event envelope fields must fail");
    assert!(error.to_string().contains("unknown field `legacy_field`"));
}
