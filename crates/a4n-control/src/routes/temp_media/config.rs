use a4n_api_contract::temp_media::TEMP_MEDIA_MAX_TTL_SECONDS;
use a4n_config::{required_non_empty_env_string, required_public_app_endpoint};

use super::super::error::ApiError;

const TEMP_MEDIA_UPLOAD_TOKEN_ENV: &str = "A4N_TEMP_MEDIA_UPLOAD_TOKEN";
const TEMP_MEDIA_TTL_ENV: &str = "A4N_TEMP_MEDIA_TTL_MINUTES";
const TEMP_MEDIA_MAX_BYTES_ENV: &str = "A4N_TEMP_MEDIA_MAX_BYTES";

pub(super) fn temp_media_upload_token() -> Result<String, ApiError> {
    required_non_empty_env_string(TEMP_MEDIA_UPLOAD_TOKEN_ENV)
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

pub(super) fn temp_media_public_base_url() -> Result<String, ApiError> {
    required_public_app_endpoint("/api/temp-media")
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

pub(super) fn temp_media_ttl_seconds() -> Result<i64, String> {
    let value =
        required_non_empty_env_string(TEMP_MEDIA_TTL_ENV).map_err(|error| error.to_string())?;
    let minutes = parse_positive_i64_config(TEMP_MEDIA_TTL_ENV, &value)?;
    if minutes > TEMP_MEDIA_MAX_TTL_SECONDS / 60 {
        return Err(format!(
            "{TEMP_MEDIA_TTL_ENV} 不能超过 {} 分钟",
            TEMP_MEDIA_MAX_TTL_SECONDS / 60
        ));
    }
    minutes
        .checked_mul(60)
        .ok_or_else(|| "临时媒体 TTL 配置超出整数范围".to_string())
}

pub(super) fn temp_media_max_bytes() -> Result<usize, String> {
    let value = required_non_empty_env_string(TEMP_MEDIA_MAX_BYTES_ENV)
        .map_err(|error| error.to_string())?;
    parse_positive_usize_config(TEMP_MEDIA_MAX_BYTES_ENV, &value)
}

pub(super) fn parse_positive_i64_config(key: &str, value: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{key} 必须是正整数"))
}

pub(super) fn parse_positive_usize_config(key: &str, value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{key} 必须是正整数"))
}
