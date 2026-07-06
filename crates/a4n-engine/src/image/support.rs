use a4n_core::error::AppError;
use base64::Engine as _;
use image::GenericImageView;
use reqwest::StatusCode;
use reqwest::header::CONTENT_TYPE;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::types::ImageGenerationResult;

const MAX_REFERENCE_IMAGE_SIDE: u32 = 512;
const MAX_REFERENCE_IMAGE_BYTES: usize = 700 * 1024;
const REFERENCE_JPEG_QUALITY: u8 = 88;
const IMAGE_CONNECT_TIMEOUT_SECS: u64 = 20;
const IMAGE_READ_TIMEOUT_SECS: u64 = 300;
const HTTP_ERROR_PREVIEW_CHARS: usize = 500;

#[derive(Debug)]
pub(super) struct DecodedInputImage {
    pub mime_type: String,
    pub format: String,
    pub bytes: Vec<u8>,
}

pub(super) fn internal_error(message: impl Into<String>) -> AppError {
    AppError::Internal(anyhow::anyhow!(message.into()))
}

pub(super) fn request_error(label: &str, error: reqwest::Error) -> AppError {
    internal_error(format!("{label}：{}", error_chain(&error)))
}

pub(super) fn image_provider_http_error(label: &str, status: StatusCode, body: &str) -> AppError {
    let details = parse_provider_error(body);
    let message = details
        .message
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::trim)
        .map(str::to_string)
        .unwrap_or_else(|| preview_chars(body, HTTP_ERROR_PREVIEW_CHARS));

    if is_content_policy_rejection(&details, body) {
        return AppError::InvalidArgument(format!(
            "{label}被内容安全策略拒绝：{message}。请修改提示词中被供应商拒绝的高风险描写后重试"
        ));
    }

    if status == StatusCode::BAD_REQUEST
        && details
            .error_type
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("invalid_request_error"))
    {
        return AppError::InvalidArgument(format!("{label}请求被供应商拒绝：{message}"));
    }

    internal_error(format!("{label}返回 {status}：{message}"))
}

pub(super) fn build_image_http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(IMAGE_CONNECT_TIMEOUT_SECS))
        .read_timeout(Duration::from_secs(IMAGE_READ_TIMEOUT_SECS))
        .pool_idle_timeout(Duration::from_secs(30))
        .http1_only()
        .build()
        .map_err(|e| internal_error(format!("图片 HTTP 客户端初始化失败：{e}")))
}

pub(super) async fn send_image_request(
    label: &str,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, AppError> {
    request.send().await.map_err(|e| request_error(label, e))
}

fn error_chain(error: &dyn Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(err) = source {
        let text = err.to_string();
        if !parts.iter().any(|part| part == &text) {
            parts.push(text);
        }
        source = err.source();
    }
    parts.join("；原因：")
}

#[derive(Debug, Default)]
struct ProviderErrorDetails {
    message: Option<String>,
    error_type: Option<String>,
    code: Option<String>,
}

fn parse_provider_error(body: &str) -> ProviderErrorDetails {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return ProviderErrorDetails::default();
    };
    let error = value.get("error").unwrap_or(&value);
    ProviderErrorDetails {
        message: string_field(error, "message"),
        error_type: string_field(error, "type"),
        code: string_field(error, "code"),
    }
}

fn string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    let raw = value.get(field)?;
    if let Some(text) = raw.as_str() {
        return non_empty_string(text);
    }
    if raw.is_null() {
        None
    } else {
        non_empty_string(&raw.to_string())
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_content_policy_rejection(details: &ProviderErrorDetails, body: &str) -> bool {
    details
        .code
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("content_policy_violation"))
        || contains_policy_rejection_hint(body)
}

fn contains_policy_rejection_hint(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("content_policy_violation")
        || (lower.contains("content policy") && lower.contains("violation"))
        || (lower.contains("safety") && (lower.contains("policy") || lower.contains("refus")))
        || body.contains("内容安全")
        || body.contains("安全策略")
        || body.contains("不能根据这个描述生成图像")
}

fn preview_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

pub(super) fn decode_base64_image(raw: &str) -> Result<Vec<u8>, AppError> {
    let b64 = raw
        .split_once(',')
        .map(|(_, body)| body)
        .unwrap_or(raw)
        .trim();
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| internal_error(format!("base64 解码失败：{e}")))
}

fn decode_data_uri_image(data_uri: &str) -> Result<DecodedInputImage, AppError> {
    let (header, body) = data_uri
        .split_once(',')
        .ok_or_else(|| internal_error("参考图 data URI 缺少 base64 数据"))?;
    let header = header.trim();
    if !header.starts_with("data:") || !header.contains(";base64") {
        return Err(internal_error("参考图必须是 data:image/*;base64,... 格式"));
    }
    let mime_type = header
        .trim_start_matches("data:")
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let format = format_from_content_type(&mime_type)
        .ok_or_else(|| internal_error(format!("不支持的参考图 MIME 类型：{mime_type}")))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body.trim())
        .map_err(|e| internal_error(format!("参考图 base64 解码失败：{e}")))?;
    let detected = detect_format_from_bytes(&bytes)?;
    if detected != format {
        return Err(internal_error(format!(
            "参考图 MIME 类型 {mime_type} 与实际字节格式 {detected} 不一致"
        )));
    }
    Ok(DecodedInputImage {
        mime_type,
        format,
        bytes,
    })
}

pub(super) fn prepare_reference_image_for_upload(
    data_uri: &str,
) -> Result<DecodedInputImage, AppError> {
    let decoded = decode_data_uri_image(data_uri)?;
    let image = image::load_from_memory(&decoded.bytes)
        .map_err(|e| internal_error(format!("参考图解码失败：{e}")))?;
    let (width, height) = image.dimensions();
    if width <= MAX_REFERENCE_IMAGE_SIDE
        && height <= MAX_REFERENCE_IMAGE_SIDE
        && decoded.bytes.len() <= MAX_REFERENCE_IMAGE_BYTES
    {
        return Ok(decoded);
    }

    let resized = image.thumbnail(MAX_REFERENCE_IMAGE_SIDE, MAX_REFERENCE_IMAGE_SIDE);
    let rgb = resized.to_rgb8();
    let mut bytes = Vec::new();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, REFERENCE_JPEG_QUALITY);
    encoder
        .encode_image(&image::DynamicImage::ImageRgb8(rgb))
        .map_err(|e| internal_error(format!("参考图重编码失败：{e}")))?;

    Ok(DecodedInputImage {
        mime_type: "image/jpeg".into(),
        format: "jpg".into(),
        bytes,
    })
}

pub(super) fn required_non_empty<'a>(value: &'a str, field: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(internal_error(format!("图片生成配置缺少 {field}")));
    }
    Ok(value)
}

pub(super) fn required_json_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, AppError> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| internal_error(format!("图片生成响应缺少 {field}")))
}

pub(super) fn required_json_string_field<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, AppError> {
    value
        .as_str()
        .ok_or_else(|| internal_error(format!("图片生成响应缺少 {field}")))
}

pub(super) async fn response_text(
    resp: reqwest::Response,
    label: &str,
) -> Result<String, AppError> {
    resp.text()
        .await
        .map_err(|e| internal_error(format!("{label}响应读取失败：{}", error_chain(&e))))
}

pub(super) async fn response_bytes(
    resp: reqwest::Response,
    label: &str,
) -> Result<Vec<u8>, AppError> {
    let status = resp.status();
    if !status.is_success() {
        let text = response_text(resp, label).await?;
        return Err(internal_error(format!(
            "{label}返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        )));
    }
    resp.bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| internal_error(format!("{label}字节读取失败：{}", error_chain(&e))))
}

pub(super) async fn download_image_url(
    client: &reqwest::Client,
    url: &str,
    declared_format: Option<&str>,
) -> Result<(Vec<u8>, String), AppError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| request_error("下载图像请求失败", e))?;
    let status = resp.status();
    if !status.is_success() {
        let text = response_text(resp, "下载图像").await?;
        return Err(internal_error(format!(
            "下载图像返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        )));
    }

    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = resp
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| internal_error(format!("下载图像字节读取失败：{}", error_chain(&e))))?;
    let format = image_format_hint(declared_format, Some(url), content_type.as_deref(), &bytes)?;
    Ok((bytes, format))
}

pub(super) async fn image_result_from_openai_payload(
    client: &reqwest::Client,
    text: &str,
    model: String,
    label: &str,
) -> Result<ImageGenerationResult, AppError> {
    let v: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| internal_error(format!("{label}响应解析失败：{e}")))?;
    let item = v
        .get("data")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .ok_or_else(|| internal_error(format!("{label}响应缺少 data[0]")))?;
    let declared_format = optional_format_from_response_item(item)?;

    if let Some(b64) = item
        .get("b64_json")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let bytes = decode_base64_image(b64)?;
        let format = image_format_hint(declared_format.as_deref(), None, None, &bytes)?;
        return Ok(ImageGenerationResult {
            format,
            bytes,
            model,
        });
    }

    if let Some(img_url) = item
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        tracing::debug!("provider 返回 URL，下载：{img_url}");
        let (bytes, format) =
            download_image_url(client, img_url, declared_format.as_deref()).await?;
        return Ok(ImageGenerationResult {
            bytes,
            format,
            model,
        });
    }

    Err(internal_error(format!(
        "{label}响应缺少 data[0].b64_json 或 data[0].url"
    )))
}

pub(super) fn normalize_image_format(value: &str) -> Result<String, AppError> {
    let format = value.trim().trim_start_matches('.').to_ascii_lowercase();
    match format.as_str() {
        "png" => Ok("png".into()),
        "jpg" | "jpeg" => Ok("jpg".into()),
        "webp" => Ok("webp".into()),
        "" => Err(internal_error("图片响应缺少 format")),
        other => Err(internal_error(format!("不支持的图片格式：{other}"))),
    }
}

pub(super) fn detect_format_from_bytes(bytes: &[u8]) -> Result<String, AppError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("png".into())
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Ok("jpg".into())
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Ok("webp".into())
    } else {
        Err(internal_error("无法识别图片字节格式"))
    }
}

pub(super) fn image_format_hint(
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

pub(super) fn optional_format_from_response_item(
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

fn format_from_url(url: &str) -> Option<String> {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .trim_end_matches('/');
    let ext = path.rsplit_once('.')?.1;
    normalize_image_format(ext).ok()
}

fn format_from_content_type(content_type: &str) -> Option<String> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    match media_type.as_str() {
        "image/png" => Some("png".into()),
        "image/jpeg" | "image/jpg" => Some("jpg".into()),
        "image/webp" => Some("webp".into()),
        _ => None,
    }
}

pub(super) fn percent_encode_query(value: &str) -> String {
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

pub(super) fn time_seed() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() % (i64::MAX as u128)) as i64)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_supported_declared_formats() {
        assert_eq!(normalize_image_format("png").unwrap(), "png");
        assert_eq!(normalize_image_format(".JPEG").unwrap(), "jpg");
        assert_eq!(normalize_image_format("webp").unwrap(), "webp");
        assert!(normalize_image_format("").is_err());
        assert!(normalize_image_format("gif").is_err());
    }

    #[test]
    fn detects_known_image_magic_without_default_format() {
        assert_eq!(
            detect_format_from_bytes(b"\x89PNG\r\n\x1a\nrest").unwrap(),
            "png"
        );
        assert_eq!(
            detect_format_from_bytes(b"\xff\xd8\xffrest").unwrap(),
            "jpg"
        );
        assert_eq!(
            detect_format_from_bytes(b"RIFFxxxxWEBPrest").unwrap(),
            "webp"
        );
        assert!(detect_format_from_bytes(b"not-an-image").is_err());
    }

    #[test]
    fn infers_image_format_from_url_content_type_or_bytes() {
        assert_eq!(
            image_format_hint(None, Some("https://example.com/a.jpeg?x=1"), None, b"").unwrap(),
            "jpg"
        );
        assert_eq!(
            image_format_hint(None, None, Some("image/webp; charset=utf-8"), b"").unwrap(),
            "webp"
        );
        assert_eq!(
            image_format_hint(None, None, None, b"\x89PNG\r\n\x1a\nrest").unwrap(),
            "png"
        );
    }

    #[test]
    fn reads_optional_format_or_mime_type_from_response_item() {
        assert_eq!(
            optional_format_from_response_item(&serde_json::json!({"format": "jpeg"})).unwrap(),
            Some("jpg".into())
        );
        assert_eq!(
            optional_format_from_response_item(&serde_json::json!({"mime_type": "image/jpeg"}))
                .unwrap(),
            Some("jpg".into())
        );
        assert_eq!(
            optional_format_from_response_item(&serde_json::json!({})).unwrap(),
            None
        );
    }

    #[test]
    fn content_policy_http_error_is_invalid_argument() {
        let body = r#"{"error":{"message":"抱歉，我不能根据这个描述生成图像，因为其中包含明确的自我伤害/流血与即将接触尖锐物体的情节描写。","type":"invalid_request_error","param":"","code":"content_policy_violation"}}"#;

        let err = image_provider_http_error("图像生成", StatusCode::BAD_REQUEST, body);

        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message.contains("内容安全策略拒绝")
                    && message.contains("自我伤害")
                    && message.contains("请修改提示词")
        ));
    }

    #[test]
    fn invalid_request_http_error_is_invalid_argument() {
        let body = r#"{"error":{"message":"size is invalid","type":"invalid_request_error","code":"bad_request"}}"#;

        let err = image_provider_http_error("图像生成", StatusCode::BAD_REQUEST, body);

        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message.contains("请求被供应商拒绝") && message.contains("size is invalid")
        ));
    }

    #[test]
    fn non_client_provider_http_error_stays_internal() {
        let body = r#"{"error":{"message":"upstream unavailable"}}"#;

        let err = image_provider_http_error("图像生成", StatusCode::BAD_GATEWAY, body);

        assert!(matches!(
            err,
            AppError::Internal(error) if error.to_string().contains("502 Bad Gateway")
        ));
    }

    #[test]
    fn decodes_data_uri_input_image() {
        let decoded = decode_data_uri_image(
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
        )
        .unwrap();

        assert_eq!(decoded.mime_type, "image/png");
        assert_eq!(decoded.format, "png");
        assert!(decoded.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn rejects_data_uri_with_mismatched_mime() {
        let err = decode_data_uri_image(
            "data:image/jpeg;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
        )
        .unwrap_err();

        assert!(err.to_string().contains("不一致"));
    }

    #[test]
    fn prepares_large_reference_image_as_compact_jpeg() {
        let image = image::RgbImage::from_pixel(1024, 1024, image::Rgb([220, 40, 30]));
        let mut png = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let data_uri = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        );

        let prepared = prepare_reference_image_for_upload(&data_uri).unwrap();

        assert_eq!(prepared.mime_type, "image/jpeg");
        assert_eq!(prepared.format, "jpg");
        assert!(prepared.bytes.starts_with(b"\xff\xd8\xff"));
        assert!(prepared.bytes.len() < MAX_REFERENCE_IMAGE_BYTES);
    }
}
