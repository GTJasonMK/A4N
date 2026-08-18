mod config;
mod delete;
mod rules;
mod serve;
mod storage;
mod upload;

use axum::http::{HeaderMap, header};

use super::error::ApiError;

pub(super) use delete::delete;
pub(super) use serve::serve;
pub(super) use upload::upload;

pub(super) fn authorize(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = config::temp_media_upload_token()?;
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthorized("缺少临时媒体访问令牌"))?;
    if token != expected {
        return Err(ApiError::forbidden("临时媒体访问令牌无效"));
    }
    Ok(())
}

#[cfg(test)]
use config::{parse_positive_i64_config, parse_positive_usize_config};
#[cfg(test)]
use rules::{
    validate_media_magic, validate_temp_media_artifact_id, validate_temp_media_file_name,
    validate_upload_file_name_hint,
};

#[cfg(test)]
mod tests;
