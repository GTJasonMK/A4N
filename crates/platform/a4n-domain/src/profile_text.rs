use serde_json::{Value, json};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProfileTextError {
    #[error("{field}不能为空")]
    Empty { field: String },
    #[error("{field}无效: {value}；允许值：primary/secondary/minor")]
    InvalidImportance { field: String, value: String },
}

pub fn required_text(value: &str, field_name: &str) -> Result<String, ProfileTextError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ProfileTextError::Empty {
            field: field_name.to_string(),
        });
    }
    Ok(value.to_string())
}

pub fn validate_importance(value: &str, field_name: &str) -> Result<String, ProfileTextError> {
    normalize_importance_value(value).map_err(|error| match error {
        ProfileTextError::Empty { .. } => ProfileTextError::Empty {
            field: field_name.to_string(),
        },
        ProfileTextError::InvalidImportance { value, .. } => ProfileTextError::InvalidImportance {
            field: field_name.to_string(),
            value,
        },
    })
}

pub fn normalize_importance_value(value: &str) -> Result<String, ProfileTextError> {
    let value = required_text(value, "重要性")?;
    if matches!(value.as_str(), "primary" | "secondary" | "minor") {
        return Ok(value);
    }
    Err(ProfileTextError::InvalidImportance {
        field: "重要性".to_string(),
        value,
    })
}

pub fn normalize_aliases(aliases: Vec<String>) -> Vec<String> {
    aliases
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn merge_profile_evidence_json(existing: &Value, incoming: &Value) -> Value {
    let values = existing
        .as_array()
        .into_iter()
        .flatten()
        .chain(incoming.as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    json!(values)
}

pub fn normalize_required_text_set(
    values: Vec<String>,
    field_name: &str,
) -> Result<Vec<String>, ProfileTextError> {
    let values = normalize_text_set(values);
    if values.is_empty() {
        return Err(ProfileTextError::Empty {
            field: field_name.to_string(),
        });
    }
    Ok(values)
}

pub fn normalize_text_set(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_importance_accepts_canonical_values_only() {
        assert_eq!(normalize_importance_value(" primary ").unwrap(), "primary");
        assert_eq!(
            normalize_importance_value("secondary").unwrap(),
            "secondary"
        );
        assert_eq!(normalize_importance_value("minor").unwrap(), "minor");
        for alias in [
            "high",
            "key",
            "medium",
            "supporting",
            "low",
            "background",
            "重要",
        ] {
            let err = normalize_importance_value(alias).expect_err("importance aliases must fail");
            assert!(matches!(err, ProfileTextError::InvalidImportance { .. }));
        }
    }

    #[test]
    fn normalize_importance_rejects_unknown_values() {
        let err = normalize_importance_value("legendary")
            .expect_err("unknown importance should remain visible");
        assert!(matches!(err, ProfileTextError::InvalidImportance { .. }));
    }

    #[test]
    fn profile_evidence_merge_normalizes_and_deduplicates_text() {
        assert_eq!(
            merge_profile_evidence_json(
                &json!([" 旧证据 ", "重复", null]),
                &json!(["重复", "", "新证据"]),
            ),
            json!(["新证据", "旧证据", "重复"])
        );
    }
}
