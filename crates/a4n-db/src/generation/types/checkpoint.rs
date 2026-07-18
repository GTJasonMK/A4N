use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::sha256_hex;

pub fn generation_checkpoint_fingerprint_sha256(
    value: &Value,
) -> Result<String, serde_json::Error> {
    let serialized = serde_json::to_string(value)?;
    Ok(sha256_hex(&serialized))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationRunCheckpointContext {
    pub workflow_run_id: Option<i64>,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub raw_response_json: Option<Value>,
    pub parsed_output_json: Option<Value>,
    pub validation_json: Option<Value>,
    pub artifact_refs_json: Option<Value>,
    pub attempt: Option<i32>,
}
