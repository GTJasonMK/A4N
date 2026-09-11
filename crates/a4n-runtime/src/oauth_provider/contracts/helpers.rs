use a4n_error::AppError;

pub fn oauth_token_error(provider: &str, body: &serde_json::Value) -> AppError {
    if let Some(error) = body["error_description"]
        .as_str()
        .map(str::trim)
        .filter(|error| !error.is_empty())
    {
        return AppError::Forbidden(format!("{provider} 令牌交换失败: {error}"));
    }

    AppError::Forbidden(format!(
        "{provider} 令牌交换失败: 响应缺少 access_token，body={body}"
    ))
}

pub fn required_non_empty_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
    context: &str,
) -> Result<&'a str, AppError> {
    value[field]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}缺少字符串字段 {field}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn token_error_uses_response_body_when_description_is_missing() {
        let err = oauth_token_error("GitHub", &json!({"error": "bad_verification_code"}));
        let message = err.to_string();

        assert!(message.contains("access_token"));
        assert!(message.contains("bad_verification_code"));
    }
}
