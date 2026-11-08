mod errors;
mod format;
mod http;
mod openai_payload;
mod reference;
#[cfg(test)]
mod tests;

pub(super) use errors::{image_provider_http_error, internal_error};
#[cfg(test)]
pub(super) use format::optional_format_from_response_item;
pub(super) use format::{
    decode_base64_image, detect_format_from_bytes, image_format_hint, normalize_image_format,
    percent_encode_query,
};
pub(super) use http::{
    build_image_http_client, download_image_url, response_bytes, response_text, send_image_request,
};
pub(super) use openai_payload::image_result_from_openai_payload;
pub(super) use reference::{
    decode_data_uri_image, prepare_reference_image_for_upload, prompt_with_reference_labels,
    reference_image_field_name,
};

pub(super) fn required_non_empty<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, a4n_domain::ProviderError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(internal_error(format!("图片生成配置缺少 {field}")));
    }
    Ok(value)
}

pub(super) fn required_json_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, a4n_domain::ProviderError> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| internal_error(format!("图片生成响应缺少 {field}")))
}

pub(super) fn required_json_string_field<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, a4n_domain::ProviderError> {
    value
        .as_str()
        .ok_or_else(|| internal_error(format!("图片生成响应缺少 {field}")))
}

fn time_seed_at(now: std::time::SystemTime) -> Result<i64, a4n_domain::ProviderError> {
    now.duration_since(std::time::UNIX_EPOCH)
        .map(|duration| (duration.as_nanos() % (i64::MAX as u128)) as i64)
        .map_err(|error| {
            internal_error(format!(
                "系统时间早于 Unix epoch，无法生成 ComfyUI 请求标识：{error}"
            ))
        })
}

pub(super) fn time_seed() -> Result<i64, a4n_domain::ProviderError> {
    time_seed_at(std::time::SystemTime::now())
}
