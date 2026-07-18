use super::errors::internal_error;
use super::format::{detect_format_from_bytes, format_from_content_type};
use a4n_core::error::AppError;
use base64::Engine as _;
use image::GenericImageView;

const MAX_REFERENCE_IMAGE_SIDE: u32 = 512;
pub(super) const MAX_REFERENCE_IMAGE_BYTES: usize = 700 * 1024;
const REFERENCE_JPEG_QUALITY: u8 = 88;

#[derive(Debug)]
pub(crate) struct DecodedInputImage {
    pub mime_type: String,
    pub format: String,
    pub bytes: Vec<u8>,
}
pub(super) fn decode_data_uri_image(data_uri: &str) -> Result<DecodedInputImage, AppError> {
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

pub(crate) fn prepare_reference_image_for_upload(
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
