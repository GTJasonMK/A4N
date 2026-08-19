//! 上传图片解码与格式纯规则。

use image::GenericImageView;

use super::{decode_uploaded_bytes, required_upload_file_name};

/// 解码后的上传图片事实。
#[derive(Debug)]
pub struct DecodedUploadedImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
    pub mime_type: &'static str,
    pub width: i32,
    pub height: i32,
    pub source_file_name: String,
}

/// 解码 base64 上传图片并校验 MIME/尺寸。
pub fn decode_uploaded_image_data(
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
    max_bytes: usize,
) -> Result<DecodedUploadedImage, String> {
    let expected_mime = normalize_upload_mime(mime_type)?;
    let bytes = decode_uploaded_bytes(data_base64, "上传图片", max_bytes)?;

    let format = image::guess_format(&bytes).map_err(|_| "上传图片不是有效图片".to_string())?;
    let (actual_mime, extension) = supported_upload_image_format(format)?;
    if actual_mime != expected_mime {
        return Err("上传图片 MIME 与真实图片格式不一致".to_string());
    }

    let decoded = image::load_from_memory_with_format(&bytes, format)
        .map_err(|err| format!("上传图片解析失败：{err}"))?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return Err("上传图片尺寸无效".to_string());
    }
    let width = i32::try_from(width).map_err(|_| "上传图片宽度超过 i32 范围".to_string())?;
    let height = i32::try_from(height).map_err(|_| "上传图片高度超过 i32 范围".to_string())?;
    let source_file_name = required_upload_file_name(file_name, "上传图片文件名")?;

    Ok(DecodedUploadedImage {
        bytes,
        extension,
        mime_type: actual_mime,
        width,
        height,
        source_file_name,
    })
}

fn normalize_upload_mime(mime_type: &str) -> Result<&'static str, String> {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/png" => Ok("image/png"),
        "image/jpeg" => Ok("image/jpeg"),
        "image/webp" => Ok("image/webp"),
        _ => Err("只支持 PNG/JPEG/WebP 图片".to_string()),
    }
}

fn supported_upload_image_format(
    format: image::ImageFormat,
) -> Result<(&'static str, &'static str), String> {
    match format {
        image::ImageFormat::Png => Ok(("image/png", "png")),
        image::ImageFormat::Jpeg => Ok(("image/jpeg", "jpg")),
        image::ImageFormat::WebP => Ok(("image/webp", "webp")),
        _ => Err("上传图片格式不支持，仅支持 PNG/JPEG/WebP".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_uploaded_image_data;

    const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    #[test]
    fn uploaded_image_decode_accepts_raw_base64() {
        let decoded = decode_uploaded_image_data(" ref.png ", "image/png", PNG_1X1, 1024 * 1024)
            .expect("valid uploaded png should decode");

        assert_eq!(decoded.source_file_name, "ref.png");
        assert_eq!(decoded.mime_type, "image/png");
        assert_eq!(decoded.extension, "png");
        assert_eq!((decoded.width, decoded.height), (1, 1));
    }

    #[test]
    fn uploaded_image_decode_rejects_empty_file_name() {
        let err = decode_uploaded_image_data("  ", "image/png", PNG_1X1, 1024 * 1024)
            .expect_err("empty source file name should fail");

        assert_eq!(err, "上传图片文件名不能为空");
    }

    #[test]
    fn uploaded_image_decode_rejects_mime_format_mismatch() {
        let err = decode_uploaded_image_data("ref.jpg", "image/jpeg", PNG_1X1, 1024 * 1024)
            .expect_err("declared jpeg with png bytes should fail");

        assert_eq!(err, "上传图片 MIME 与真实图片格式不一致");
    }
}
