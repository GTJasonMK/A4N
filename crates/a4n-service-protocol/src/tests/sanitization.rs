use serde_json::json;

use crate::{
    contains_unredacted_sensitive_value, redact_sensitive_message, redact_sensitive_value,
};

#[test]
fn redacts_nested_sensitive_fields() {
    let mut value = json!({
        "api_key": "sk-test",
        "nested": {
            "upload_token": "token",
            "base_url": "https://provider.example/v1",
            "baseUrl": "https://provider.example/v2",
            "visible": "ok"
        }
    });

    redact_sensitive_value(&mut value);

    assert_eq!(value["api_key"], "[REDACTED]");
    assert_eq!(value["nested"]["upload_token"], "[REDACTED]");
    assert_eq!(value["nested"]["base_url"], "[REDACTED]");
    assert_eq!(value["nested"]["baseUrl"], "[REDACTED]");
    assert_eq!(value["nested"]["visible"], "ok");
    assert!(!contains_unredacted_sensitive_value(&value));
}

#[test]
fn detects_unredacted_sensitive_fields_without_flagging_empty_values() {
    assert!(contains_unredacted_sensitive_value(&json!({
        "nested": [{ "api_key": "sk-test" }]
    })));
    assert!(contains_unredacted_sensitive_value(&json!({
        "baseUrl": "https://provider.example/v1"
    })));
    assert!(!contains_unredacted_sensitive_value(&json!({
        "api_key": null,
        "upload_token": " ",
        "base_url": "[REDACTED]",
        "visible": "https://public.example/docs"
    })));
}

#[test]
fn redacts_sensitive_json_message() {
    let message = r#"{"base_url":"https://provider.example/v1","visible":"ok"}"#;

    let redacted = redact_sensitive_message(message);

    assert!(redacted.contains(r#""base_url":"[REDACTED]""#));
    assert!(redacted.contains(r#""visible":"ok""#));
    assert!(!redacted.contains("provider.example"));
}

#[test]
fn redacts_sensitive_plain_text_key_values_and_urls() {
    let message =
        "provider failed: base_url=https://provider.example/v1 api_key=sk-test, visible=ok";

    let redacted = redact_sensitive_message(message);

    assert!(redacted.contains("base_url=[REDACTED]"));
    assert!(redacted.contains("api_key=[REDACTED]"));
    assert!(redacted.contains("visible=ok"));
    assert!(!redacted.contains("provider.example"));
    assert!(!redacted.contains("sk-test"));
}

#[test]
fn redacts_bare_url_in_plain_text_message() {
    let message = "request failed at https://provider.example/v1/models";

    let redacted = redact_sensitive_message(message);

    assert_eq!(redacted, "request failed at [REDACTED_URL]");
}
