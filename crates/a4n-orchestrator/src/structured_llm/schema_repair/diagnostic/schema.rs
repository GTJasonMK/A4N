use a4n_core::error::AppError;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(in crate::structured_llm::schema_repair) fn structured_schema_sha256(
    schema: &Value,
) -> Result<String, AppError> {
    let schema_text = serde_json::to_vec(schema)?;
    Ok(format!("{:x}", Sha256::digest(&schema_text)))
}
