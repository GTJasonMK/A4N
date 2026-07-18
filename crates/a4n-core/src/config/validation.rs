use crate::error::AppError;

pub fn required_config_value(value: &str, kind: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{kind}缺少 {field}")));
    }
    Ok(value)
}

pub fn normalize_optional_config_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn positive_i32(value: i32, field: &str) -> Result<i32, AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

pub fn normalize_config_name(value: impl AsRef<str>, kind: &str) -> Result<String, AppError> {
    let value = value.as_ref().trim().to_string();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{kind}名称不能为空")));
    }
    Ok(value)
}

pub fn app_error_plain_message(err: AppError) -> String {
    match err {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_shared_config_text_and_positive_values() {
        assert_eq!(
            normalize_config_name("  图片配置  ", "配置").expect("non-empty name"),
            "图片配置"
        );
        assert_eq!(
            required_config_value("  model  ", "图片配置", "model").expect("non-empty value"),
            "model"
        );
        assert_eq!(
            normalize_optional_config_value(Some("  secret  ".to_string())),
            Some("secret".to_string())
        );
        assert_eq!(positive_i32(1, "limit").expect("positive value"), 1);
    }

    #[test]
    fn rejects_empty_required_values_and_non_positive_numbers() {
        assert_eq!(
            normalize_config_name(" ", "图片配置")
                .expect_err("empty name")
                .to_string(),
            "参数无效：图片配置名称不能为空"
        );
        assert_eq!(
            required_config_value(" ", "图片配置", "model")
                .expect_err("empty field")
                .to_string(),
            "参数无效：图片配置缺少 model"
        );
        assert_eq!(
            positive_i32(0, "图片配置 max_concurrent")
                .expect_err("zero value")
                .to_string(),
            "参数无效：图片配置 max_concurrent 必须大于 0"
        );
    }
}
