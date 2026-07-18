use crate::error::AppError;
use std::collections::BTreeSet;

pub fn required_text(value: &str, field_name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field_name}不能为空")));
    }
    Ok(value.to_string())
}

pub fn validate_importance(value: &str, field_name: &str) -> Result<String, AppError> {
    normalize_importance_value(value).map_err(|_| {
        AppError::InvalidArgument(format!(
            "{field_name}无效: {}；允许值：primary/secondary/minor",
            value.trim()
        ))
    })
}

pub fn normalize_importance_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "重要性")?;
    if matches!(value.as_str(), "primary" | "secondary" | "minor") {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "重要性无效: {value}；允许值：primary/secondary/minor"
    )))
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

pub fn normalize_required_text_set(
    values: Vec<String>,
    field_name: &str,
) -> Result<Vec<String>, AppError> {
    let values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field_name}不能为空")));
    }
    Ok(values)
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
            assert!(
                matches!(err, AppError::InvalidArgument(message) if message.contains("重要性无效")),
                "{alias} should be rejected"
            );
        }
    }

    #[test]
    fn normalize_importance_rejects_unknown_values() {
        let err = normalize_importance_value("legendary")
            .expect_err("unknown importance should remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("重要性无效"))
        );
    }
}
