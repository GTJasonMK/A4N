pub fn reject_deprecated_output_fields(
    value: &serde_json::Value,
    path: &str,
    fields: &[&str],
    expected_field: &str,
) -> Result<(), String> {
    reject_deprecated_fields(value, path, fields, expected_field, "输出")
}

pub fn reject_deprecated_reference_fields(
    value: &serde_json::Value,
    path: &str,
    fields: &[&str],
    expected_field: &str,
) -> Result<(), String> {
    reject_deprecated_fields(value, path, fields, expected_field, "引用")
}

fn reject_deprecated_fields(
    value: &serde_json::Value,
    path: &str,
    fields: &[&str],
    expected_field: &str,
    requirement: &str,
) -> Result<(), String> {
    for key in fields {
        if value.get(key).is_some() {
            return Err(format!(
                "{path}.{key} 已废弃；必须{requirement} {expected_field}"
            ));
        }
    }
    Ok(())
}
