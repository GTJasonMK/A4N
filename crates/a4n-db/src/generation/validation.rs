use super::*;
use serde::Serialize;

pub use a4n_domain::generation::{
    canonical_json_bytes, canonical_json_sha256, canonical_json_text,
};

pub(super) fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, GenerationDbError> {
    let value = value.trim();
    if value.is_empty() {
        Err(GenerationDbError::InvalidArgument(format!(
            "{label} 不能为空"
        )))
    } else {
        Ok(value)
    }
}

pub(super) fn required_owned_text(value: String, label: &str) -> Result<String, GenerationDbError> {
    Ok(required_text(&value, label)?.to_string())
}

pub(super) fn normalize_optional_owned_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

pub(super) fn normalize_one_of<'a>(
    value: &'a str,
    label: &str,
    allowed: &[&str],
) -> Result<&'a str, GenerationDbError> {
    let value = value.trim();
    if allowed.contains(&value) {
        Ok(value)
    } else {
        Err(GenerationDbError::InvalidArgument(format!(
            "{label} 无效: {value}"
        )))
    }
}

pub(super) fn validate_positive_id(value: i64, label: &str) -> Result<(), GenerationDbError> {
    if value > 0 {
        Ok(())
    } else {
        Err(GenerationDbError::InvalidArgument(format!(
            "{label} 必须为正数"
        )))
    }
}

pub(in crate::generation) fn verify_canonical_json_sha256<T>(
    value: &T,
    expected_sha256: &str,
) -> Result<bool, serde_json::Error>
where
    T: Serialize,
{
    Ok(canonical_json_sha256(value)? == expected_sha256)
}
