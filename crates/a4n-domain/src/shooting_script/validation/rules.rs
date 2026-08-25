use std::collections::HashSet;

use crate::shooting_script::ShootingScriptValidationError;

pub(super) fn positive_index(value: u32, path: &str) -> Result<(), ShootingScriptValidationError> {
    if value == 0 {
        return invalid(path, "必须大于 0");
    }
    Ok(())
}

pub(super) fn require_index(
    value: u32,
    offset: usize,
    path: &str,
) -> Result<(), ShootingScriptValidationError> {
    let expected = u32::try_from(offset + 1).map_err(|_| ShootingScriptValidationError {
        path: path.to_string(),
        message: "索引超出 u32 范围".to_string(),
    })?;
    if value != expected {
        return invalid(
            path,
            format!("必须从 1 开始连续递增，期望 {expected}，实际 {value}"),
        );
    }
    Ok(())
}

pub(super) fn required_text(value: &str, path: &str) -> Result<(), ShootingScriptValidationError> {
    if value.trim().is_empty() {
        invalid(path, "不能为空")
    } else {
        Ok(())
    }
}

pub(super) fn non_empty_texts(
    values: &[String],
    path: &str,
) -> Result<(), ShootingScriptValidationError> {
    if values.is_empty() {
        return invalid(path, "不能为空");
    }
    let mut seen = HashSet::new();
    for value in values {
        required_text(value, path)?;
        if !seen.insert(value.as_str()) {
            return invalid(path, format!("包含重复值: {value}"));
        }
    }
    Ok(())
}

pub(super) fn is_subset(values: &[String], parent: &[String]) -> bool {
    let parent = parent.iter().map(String::as_str).collect::<HashSet<_>>();
    values.iter().all(|value| parent.contains(value.as_str()))
}

pub(super) fn invalid<T>(
    path: impl Into<String>,
    message: impl Into<String>,
) -> Result<T, ShootingScriptValidationError> {
    Err(ShootingScriptValidationError {
        path: path.into(),
        message: message.into(),
    })
}
