use super::errors::internal_error;
use a4n_domain::ProviderError as AppError;
use a4n_domain::media::{
    ImageUploadFormat, image_upload_format_from_extension, image_upload_format_from_mime,
};
use base64::Engine as _;
pub(crate) fn decode_base64_image(raw: &str) -> Result<Vec<u8>, AppError> {
    let b64 = raw
        .split_once(',')
        .map(|(_, body)| body)
        .unwrap_or(raw)
        .trim();
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| internal_error(format!("base64 解码失败：{e}")))
}
pub(crate) fn normalize_image_format(value: &str) -> Result<String, AppError> {
    let format = value.trim().trim_start_matches('.').to_ascii_lowercase();
    if format.is_empty() {
        return Err(internal_error("图片响应缺少 format"));
    }
    image_upload_format_from_extension(&format)
        .map(|image_format| image_format.extension.to_string())
        .ok_or_else(|| internal_error(format!("不支持的图片格式：{format}")))
}

pub(crate) fn detect_format_from_bytes(bytes: &[u8]) -> Result<String, AppError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(ImageUploadFormat::PNG.extension.into())
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Ok(ImageUploadFormat::JPEG.extension.into())
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Ok(ImageUploadFormat::WEBP.extension.into())
    } else {
        Err(internal_error("无法识别图片字节格式"))
    }
}

pub(crate) fn image_format_hint(
    declared_format: Option<&str>,
    url: Option<&str>,
    content_type: Option<&str>,
    bytes: &[u8],
) -> Result<String, AppError> {
    if let Some(format) = declared_format {
        return normalize_image_format(format);
    }
    if let Some(url) = url.and_then(format_from_url) {
        return Ok(url);
    }
    if let Some(format) = content_type.and_then(format_from_content_type) {
        return Ok(format);
    }
    detect_format_from_bytes(bytes)
}

pub(crate) fn optional_format_from_response_item(
    item: &serde_json::Value,
) -> Result<Option<String>, AppError> {
    if let Some(format) = item
        .get("format")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return normalize_image_format(format).map(Some);
    }
    if let Some(mime) = item
        .get("mime_type")
        .or_else(|| item.get("content_type"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let format = format_from_content_type(mime)
            .ok_or_else(|| internal_error(format!("不支持的图片 MIME 类型：{mime}")))?;
        return Ok(Some(format));
    }
    Ok(None)
}

pub(super) fn format_from_url(url: &str) -> Option<String> {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .trim_end_matches('/');
    let ext = path.rsplit_once('.')?.1;
    normalize_image_format(ext).ok()
}

pub(super) fn format_from_content_type(content_type: &str) -> Option<String> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    let mime = if media_type == "image/jpg" {
        ImageUploadFormat::JPEG.mime_type
    } else {
        &media_type
    };
    image_upload_format_from_mime(mime).map(|image_format| image_format.extension.into())
}

pub(crate) fn percent_encode_query(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
