use std::collections::BTreeSet;

use super::validation::ProfileTextError;

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
