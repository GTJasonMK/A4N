//! Pure configuration value normalization shared by adapters.

use url::{Host, Url};

pub fn required_config_value(value: &str, kind: &str, field: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("{kind}缺少 {field}"));
    }
    Ok(value)
}

pub fn validate_http_base_url(
    value: &str,
    kind: &str,
    api_key: Option<&str>,
) -> Result<String, String> {
    let value = required_config_value(value, kind, "base_url")?;
    let url =
        Url::parse(&value).map_err(|error| format!("{kind} base_url 不是有效 URL：{error}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err(format!(
            "{kind} base_url 必须是有效的 http:// 或 https:// URL"
        ));
    }
    let has_credentials = api_key.is_some_and(|key| !key.trim().is_empty())
        || !url.username().is_empty()
        || url.password().is_some();
    if url.scheme() == "http" && has_credentials && !is_loopback_host(url.host()) {
        return Err(format!(
            "携带凭据的远程 {kind} Base URL 必须使用 HTTPS；HTTP 重定向会丢失 Authorization，且明文传输可能泄露密钥。HTTP 仅允许 localhost 或回环地址"
        ));
    }
    Ok(value)
}

pub fn normalize_optional_config_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn positive_i32(value: i32, field: &str) -> Result<i32, String> {
    if value <= 0 {
        return Err(format!("{field} 必须大于 0"));
    }
    Ok(value)
}

pub fn normalize_config_name(value: impl AsRef<str>, kind: &str) -> Result<String, String> {
    let value = value.as_ref().trim().to_string();
    if value.is_empty() {
        return Err(format!("{kind}名称不能为空"));
    }
    Ok(value)
}

fn is_loopback_host(host: Option<Host<&str>>) -> bool {
    match host {
        Some(Host::Domain(host)) => {
            host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost")
        }
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_http_base_url;

    #[test]
    fn http_base_urls_require_http_schemes_and_hosts() {
        for value in ["not a URL", "ftp://example.com", "http://["] {
            assert!(
                validate_http_base_url(value, "配置", None).is_err(),
                "{value}"
            );
        }
        assert_eq!(
            validate_http_base_url(" https://api.example/v1 ", "配置", None).unwrap(),
            "https://api.example/v1"
        );
    }

    #[test]
    fn credentials_are_rejected_over_remote_http_but_allowed_on_loopback() {
        assert!(
            validate_http_base_url("http://api.example/v1", "LLM配置", Some("key"))
                .unwrap_err()
                .contains("必须使用 HTTPS")
        );
        assert!(
            validate_http_base_url("http://user:pass@api.example/v1", "LLM配置", None).is_err()
        );
        for value in [
            "http://localhost:8080/v1",
            "http://127.0.0.1:8080/v1",
            "http://[::1]:8080/v1",
        ] {
            assert!(
                validate_http_base_url(value, "LLM配置", Some("key")).is_ok(),
                "{value}"
            );
        }
        assert!(validate_http_base_url("http://api.example/v1", "LLM配置", None).is_ok());
    }
}
