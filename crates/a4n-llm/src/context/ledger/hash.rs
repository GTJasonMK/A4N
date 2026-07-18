use super::super::contracts::CONTEXT_MANIFEST_SCHEMA_VERSION;
use super::super::planner::canonical_json_text;
use a4n_core::error::AppError;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::Value;

pub(super) fn context_config_hash(
    pack_kind: &str,
    config_identity_json: &Value,
) -> Result<String, AppError> {
    let canonical_identity = canonical_json_text(&serde_json::json!({
        "schema_version": CONTEXT_MANIFEST_SCHEMA_VERSION,
        "pack_kind": pack_kind,
        "config_identity": config_identity_json
    }))?;
    Ok(source_content_sha256(&canonical_identity))
}
