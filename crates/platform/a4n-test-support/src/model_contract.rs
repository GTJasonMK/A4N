use a4n_domain::json_contract::OutputSchemaValidator;
use serde_json::{Value, json};

pub const VALID_VIDEO_MP4: &[u8] = include_bytes!("../fixtures/model/video.mp4");
pub const AUDIO_ONLY_MP4: &[u8] = include_bytes!("../fixtures/model/audio-only.mp4");

/// Export the production-owned schema for independent mutation/engine checks.
pub fn emit_schema(call_id: &str, schema: Value) {
    assert!(!call_id.trim().is_empty());
    OutputSchemaValidator::new(&schema).expect("production schema must compile");
    println!(
        "A4N_MODEL_SCHEMA:{}",
        json!({"call_id":call_id,"schema":schema})
    );
}

pub fn emit_request_schema(call_id: &str, schema: Value, example: Value) {
    OutputSchemaValidator::new(&schema)
        .unwrap()
        .validate(&example)
        .expect("actual request fixture must satisfy its contract");
    println!(
        "A4N_MODEL_SCHEMA:{}",
        json!({"call_id":call_id,"schema":schema,"example":example,"kind":"request"})
    );
}

pub fn assert_case(
    call_id: &str,
    adapter: &str,
    variant: &str,
    scenario: &str,
    expected: Value,
    actual: Value,
    proof: Value,
) {
    assert_eq!(
        actual, expected,
        "{call_id}/{adapter}/{variant}/{scenario}: {proof}"
    );
    assert!(proof.is_object() && !proof.as_object().unwrap().is_empty());
    println!(
        "A4N_MODEL_CASE:{}",
        json!({"case_id":format!("{call_id}/{adapter}/{variant}/{scenario}"),"call_id":call_id,"adapter":adapter,"variant":variant,"scenario":scenario,"status":"passed","assertions":1,"expected":expected,"actual":actual,"proof":proof})
    );
}
