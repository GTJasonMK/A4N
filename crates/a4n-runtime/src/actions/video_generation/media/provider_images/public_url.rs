use crate::actions::video_generation::media::types::{
    DEFAULT_MEDIA_TOKEN_TTL_MINUTES, MEDIA_TOKEN_TTL_ENV,
};
use crate::jwt;
use a4n_core::config::env::optional_non_empty_env_string;
use a4n_core::models::ai::generated_image;
use a4n_orchestrator::video::encode_query_component;
use std::net::IpAddr;

pub(in crate::actions::video_generation::media) fn image_file_to_provider_media_url(
    jwt_secret: &str,
    user_id: i64,
    base: &str,
    image: &generated_image::Model,
) -> Result<String, String> {
    let media_path = image.file_path.trim();
    let token = jwt::create_media_token(
        &user_id.to_string(),
        media_path,
        jwt_secret,
        media_token_ttl_minutes()?,
    )
    .map_err(|err| err.to_string())?;
    Ok(format!(
        "{base}?path={}&media_token={}",
        encode_query_component(media_path),
        encode_query_component(&token)
    ))
}

fn media_token_ttl_minutes() -> Result<u64, String> {
    let value =
        optional_non_empty_env_string(MEDIA_TOKEN_TTL_ENV).map_err(|error| error.to_string())?;
    parse_media_token_ttl(value.as_deref())
}

fn parse_media_token_ttl(value: Option<&str>) -> Result<u64, String> {
    let Some(value) = value else {
        return Ok(DEFAULT_MEDIA_TOKEN_TTL_MINUTES);
    };
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{MEDIA_TOKEN_TTL_ENV} 必须是正整数"))
}

pub(in crate::actions::video_generation::media) fn public_media_base_url() -> Result<String, String>
{
    let raw = if let Some(value) = optional_non_empty_env_string("A4N_PUBLIC_MEDIA_BASE_URL")
        .map_err(|error| error.to_string())?
    {
        value
    } else if let Some(value) =
        optional_non_empty_env_string("A4N_PUBLIC_APP_URL").map_err(|error| error.to_string())?
    {
        format!("{}/api/media", value.trim_end_matches('/'))
    } else {
        return Err(
            "当前视频图片输入传输模式为 public_url，需要公网媒体入口：请设置 A4N_PUBLIC_APP_URL 或 A4N_PUBLIC_MEDIA_BASE_URL；本地生成通常应使用默认 auto/inline_data_url"
                .to_string(),
        );
    };
    normalize_public_media_url(&raw)
}

fn normalize_public_media_url(raw: &str) -> Result<String, String> {
    let value = raw.trim().trim_end_matches('/').to_string();
    let url = reqwest::Url::parse(&value).map_err(|error| format!("公网媒体入口无效: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("公网媒体入口必须是 http:// 或 https:// URL".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("公网媒体入口不能包含用户名或密码".to_string());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("公网媒体入口不能包含查询参数或 fragment".to_string());
    }
    let host = url.host_str().expect("host checked");
    let normalized_host = host.trim_end_matches('.');
    let ip_host = normalized_host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(normalized_host);
    let forbidden_host = normalized_host.eq_ignore_ascii_case("localhost")
        || ip_host
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback() || ip.is_unspecified());
    if forbidden_host {
        return Err("公网媒体入口不能使用 localhost、loopback 或 unspecified 地址".to_string());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_token_ttl_rejects_invalid_values_instead_of_defaulting() {
        assert_eq!(
            parse_media_token_ttl(None).unwrap(),
            DEFAULT_MEDIA_TOKEN_TTL_MINUTES
        );
        assert_eq!(parse_media_token_ttl(Some("30")).unwrap(), 30);
        assert!(parse_media_token_ttl(Some("0")).is_err());
        assert!(parse_media_token_ttl(Some("invalid")).is_err());
    }

    #[test]
    fn public_media_url_validates_parsed_host_and_contract() {
        assert_eq!(
            normalize_public_media_url("https://media.example.com/a4n/api/media/").unwrap(),
            "https://media.example.com/a4n/api/media"
        );
        for value in [
            "ftp://media.example.com/media",
            "https://localhost/media",
            "https://127.0.0.10/media",
            "https://[::1]/media",
            "https://[::]/media",
            "https://user:secret@media.example.com/media",
            "https://media.example.com/media?token=x",
        ] {
            assert!(normalize_public_media_url(value).is_err(), "{value}");
        }
    }
}
