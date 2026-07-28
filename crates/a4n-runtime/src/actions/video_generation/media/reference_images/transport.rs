use crate::actions::video_generation::media::types::{
    REMOTE_TEMP_UPLOAD_TOKEN_ENV, REMOTE_TEMP_UPLOAD_URL_ENV,
};

pub(crate) fn remote_temp_upload_configured() -> Result<bool, String> {
    let url = remote_temp_upload_env(REMOTE_TEMP_UPLOAD_URL_ENV)?;
    let token = remote_temp_upload_env(REMOTE_TEMP_UPLOAD_TOKEN_ENV)?;
    remote_temp_upload_configured_values(url.as_deref(), token.as_deref())
}

fn remote_temp_upload_configured_values(
    url: Option<&str>,
    token: Option<&str>,
) -> Result<bool, String> {
    match (url, token) {
        (None, None) => Ok(false),
        (Some(_), Some(_)) => Ok(true),
        _ => Err(format!(
            "{REMOTE_TEMP_UPLOAD_URL_ENV} 与 {REMOTE_TEMP_UPLOAD_TOKEN_ENV} 必须同时设置"
        )),
    }
}

fn remote_temp_upload_env(key: &str) -> Result<Option<String>, String> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(format!("{key} 不能为空"))
            } else {
                Ok(Some(value))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("读取 {key} 失败：{error}")),
    }
}

pub(crate) fn remote_temp_upload_config() -> Result<(String, String), String> {
    let upload_url = remote_temp_upload_env(REMOTE_TEMP_UPLOAD_URL_ENV)?.ok_or_else(|| {
        format!(
            "{REMOTE_TEMP_UPLOAD_URL_ENV} 不能为空，remote_temp_upload 需要远端临时媒体上传地址"
        )
    })?;
    let token = remote_temp_upload_env(REMOTE_TEMP_UPLOAD_TOKEN_ENV)?.ok_or_else(|| {
        format!(
            "{REMOTE_TEMP_UPLOAD_TOKEN_ENV} 不能为空，remote_temp_upload 需要远端临时媒体上传令牌"
        )
    })?;
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
    fn remote_upload_configuration_requires_both_values() {
        assert!(!remote_temp_upload_configured_values(None, None).unwrap());
        assert!(
            remote_temp_upload_configured_values(Some("https://upload.test"), Some("token"))
                .unwrap()
        );
        assert!(remote_temp_upload_configured_values(Some("https://upload.test"), None).is_err());
        assert!(remote_temp_upload_configured_values(None, Some("token")).is_err());
    }

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
