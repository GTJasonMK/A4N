use crate::actions::video_generation::media::types::{
    REMOTE_TEMP_UPLOAD_TOKEN_ENV, REMOTE_TEMP_UPLOAD_URL_ENV,
};
use a4n_config::required_non_empty_env_string;

pub(crate) fn remote_temp_upload_config() -> Result<(String, String), String> {
    let upload_url = required_non_empty_env_string(REMOTE_TEMP_UPLOAD_URL_ENV)
        .map_err(|error| error.to_string())?;
    let token = required_non_empty_env_string(REMOTE_TEMP_UPLOAD_TOKEN_ENV)
        .map_err(|error| error.to_string())?;
    let upload_url = validate_remote_temp_upload_url(&upload_url)?;
    Ok((upload_url, token))
}

fn validate_remote_temp_upload_url(value: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(value)
        .map_err(|error| format!("{REMOTE_TEMP_UPLOAD_URL_ENV} 无效：{error}"))?;
    let Some(host) = url.host_str() else {
        return Err(format!(
            "{REMOTE_TEMP_UPLOAD_URL_ENV} 必须是包含主机名的 http:// 或 https:// URL"
        ));
    };
    if !matches!(url.scheme(), "http" | "https")
        || (url.scheme() == "http" && !is_loopback_host(host))
    {
        return Err(format!(
            "{REMOTE_TEMP_UPLOAD_URL_ENV} 仅允许 HTTPS；HTTP 只能用于 loopback"
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!(
            "{REMOTE_TEMP_UPLOAD_URL_ENV} 不能在 URL 中包含用户名或密码"
        ));
    }
    if url.fragment().is_some() {
        return Err(format!("{REMOTE_TEMP_UPLOAD_URL_ENV} 不能包含 fragment"));
    }
    Ok(url.to_string())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_upload_url_rejects_non_http_and_credentials() {
        assert!(validate_remote_temp_upload_url("file:///tmp/upload").is_err());
        assert!(validate_remote_temp_upload_url("http://example.com/upload").is_err());
        assert!(validate_remote_temp_upload_url("http://127.0.0.1:8080/upload").is_ok());
        assert!(validate_remote_temp_upload_url("https://user:pass@example.com/upload").is_err());
        assert!(validate_remote_temp_upload_url("https://example.com/upload#fragment").is_err());
        assert_eq!(
            validate_remote_temp_upload_url("https://example.com/upload?scope=video").unwrap(),
            "https://example.com/upload?scope=video"
        );
    }
}
