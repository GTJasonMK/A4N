mod errors;
mod format;
mod http;
mod openai_payload;
mod reference;
#[cfg(test)]
mod tests;

pub(super) use errors::{image_provider_http_error, internal_error};
pub(super) use format::{
    decode_base64_image, detect_format_from_bytes, normalize_image_format, percent_encode_query,
};
#[cfg(test)]
pub(super) use format::{image_format_hint, optional_format_from_response_item};
pub(super) use http::{
    build_image_http_client, download_image_url, response_bytes, response_text, send_image_request,
};
pub(super) use openai_payload::image_result_from_openai_payload;
pub(super) use reference::prepare_reference_image_for_upload;

pub(super) fn required_non_empty<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, a4n_core::error::AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(internal_error(format!("图片生成配置缺少 {field}")));
    }
    Ok(value)
}

pub(super) fn required_json_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, a4n_core::error::AppError> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| internal_error(format!("图片生成响应缺少 {field}")))
}

pub(super) fn required_json_string_field<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, a4n_core::error::AppError> {
    value
        .as_str()
        .ok_or_else(|| internal_error(format!("图片生成响应缺少 {field}")))
}

pub(super) fn time_seed() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() % (i64::MAX as u128)) as i64)
        .unwrap_or(1)
}
