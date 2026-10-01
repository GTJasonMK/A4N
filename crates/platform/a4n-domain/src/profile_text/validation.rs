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
