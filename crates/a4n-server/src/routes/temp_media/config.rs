use a4n_core::config::env::optional_non_empty_env_string;
use axum::http::Uri;

use super::super::error::ApiError;

const TEMP_MEDIA_UPLOAD_TOKEN_ENV: &str = "A4N_TEMP_MEDIA_UPLOAD_TOKEN";
const TEMP_MEDIA_PUBLIC_BASE_URL_ENV: &str = "A4N_TEMP_MEDIA_PUBLIC_BASE_URL";
const PUBLIC_APP_URL_ENV: &str = "A4N_PUBLIC_APP_URL";
const TEMP_MEDIA_TTL_ENV: &str = "A4N_TEMP_MEDIA_TTL_MINUTES";
const TEMP_MEDIA_MAX_BYTES_ENV: &str = "A4N_TEMP_MEDIA_MAX_BYTES";
const DEFAULT_TEMP_MEDIA_TTL_MINUTES: i64 = 360;
const DEFAULT_TEMP_MEDIA_MAX_BYTES: usize = 20 * 1024 * 1024;
const MAX_TEMP_MEDIA_TTL_SECONDS: i64 = 24 * 60 * 60;

pub(super) fn temp_media_upload_token() -> Result<String, ApiError> {
    optional_non_empty_env_string(TEMP_MEDIA_UPLOAD_TOKEN_ENV)
        .map_err(|error| ApiError::bad_request(error.to_string()))?
        .ok_or_else(|| ApiError::bad_request("A4N_TEMP_MEDIA_UPLOAD_TOKEN 不能为空"))
}

pub(super) fn temp_media_public_base_url() -> Result<String, ApiError> {
    let raw = if let Some(value) = optional_non_empty_env_string(TEMP_MEDIA_PUBLIC_BASE_URL_ENV)
        .map_err(|error| ApiError::bad_request(error.to_string()))?
    {
        value
    } else if let Some(value) = optional_non_empty_env_string(PUBLIC_APP_URL_ENV)
        .map_err(|error| ApiError::bad_request(error.to_string()))?
    {
        format!("{}/api/temp-media", value.trim_end_matches('/'))
    } else {
        return Err(ApiError::bad_request(
            "A4N_TEMP_MEDIA_PUBLIC_BASE_URL 或 A4N_PUBLIC_APP_URL 不能为空",
        ));
    };
    normalize_temp_media_public_url(&raw).map_err(ApiError::bad_request)
}

pub(super) fn normalize_temp_media_public_url(raw: &str) -> Result<String, String> {
    let value = raw.trim().trim_end_matches('/').to_string();
    let uri = value
        .parse::<Uri>()
        .map_err(|error| format!("临时媒体公网地址无效: {error}"))?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err("临时媒体公网地址必须是 http:// 或 https:// URL".to_string());
    }
    if uri
        .authority()
        .is_some_and(|authority| authority.as_str().contains('@'))
    {
        return Err("临时媒体公网地址不能包含用户名或密码".to_string());
    }
    if uri.query().is_some() {
        return Err("临时媒体公网地址不能包含查询参数".to_string());
    }
    Ok(value)
}

pub(super) fn temp_media_ttl_minutes() -> Result<i64, String> {
    let value =
        optional_non_empty_env_string(TEMP_MEDIA_TTL_ENV).map_err(|error| error.to_string())?;
    let minutes = parse_positive_i64_config(
        TEMP_MEDIA_TTL_ENV,
        value.as_deref(),
        DEFAULT_TEMP_MEDIA_TTL_MINUTES,
    )?;
    if minutes > MAX_TEMP_MEDIA_TTL_SECONDS / 60 {
        return Err(format!(
            "{TEMP_MEDIA_TTL_ENV} 不能超过 {} 分钟",
            MAX_TEMP_MEDIA_TTL_SECONDS / 60
        ));
    }
    Ok(minutes)
}

pub(super) fn temp_media_effective_ttl_seconds(requested: i64) -> Result<i64, String> {
    let configured = temp_media_ttl_minutes()?;
    effective_temp_media_ttl_seconds(requested, configured)
}

pub(super) fn effective_temp_media_ttl_seconds(
    requested: i64,
    configured_minutes: i64,
) -> Result<i64, String> {
    if !(1..=MAX_TEMP_MEDIA_TTL_SECONDS).contains(&requested) {
        return Err(format!(
            "expires_in_seconds 必须在 1 到 {MAX_TEMP_MEDIA_TTL_SECONDS} 之间"
        ));
    }
    let configured = configured_minutes
        .checked_mul(60)
        .ok_or_else(|| "临时媒体 TTL 配置超出整数范围".to_string())?;
    if !(1..=MAX_TEMP_MEDIA_TTL_SECONDS).contains(&configured) {
        return Err(format!(
            "服务端临时媒体 TTL 必须在 1 到 {MAX_TEMP_MEDIA_TTL_SECONDS} 秒之间"
        ));
    }
    if requested < configured {
        return Err(format!(
            "expires_in_seconds 不能短于服务端临时媒体 TTL：{requested} < {configured}"
        ));
    }
    Ok(configured)
}

pub(super) fn temp_media_max_bytes() -> Result<usize, String> {
    let value = optional_non_empty_env_string(TEMP_MEDIA_MAX_BYTES_ENV)
        .map_err(|error| error.to_string())?;
    parse_positive_usize_config(
        TEMP_MEDIA_MAX_BYTES_ENV,
        value.as_deref(),
        DEFAULT_TEMP_MEDIA_MAX_BYTES,
    )
}

pub(super) fn parse_positive_i64_config(
    key: &str,
    value: Option<&str>,
    default: i64,
) -> Result<i64, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{key} 必须是正整数"))
}

pub(super) fn parse_positive_usize_config(
    key: &str,
    value: Option<&str>,
    default: usize,
) -> Result<usize, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{key} 必须是正整数"))
}
