use base64::Engine as _;
use image::GenericImageView;

#[derive(Debug)]
pub(crate) struct DecodedUploadedImage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) extension: &'static str,
    pub(crate) mime_type: &'static str,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) file_size: i64,
    pub(crate) source_file_name: String,
}

pub(crate) fn decode_uploaded_image_data(
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
    max_bytes: usize,
) -> Result<DecodedUploadedImage, String> {
    let expected_mime = normalize_upload_mime(mime_type)?;
    let raw = strip_data_url_prefix(data_base64).trim();
    if raw.is_empty() {
        return Err("上传图片数据不能为空".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("上传图片 base64 解码失败：{err}"))?;
    if bytes.is_empty() {
        return Err("上传图片不能为空".to_string());
    }
    if bytes.len() > max_bytes {
        return Err(format!("上传图片超过 {}MB", max_bytes / 1024 / 1024));
    }

    let format = image::guess_format(&bytes).map_err(|_| "上传图片不是有效图片".to_string())?;
    let (actual_mime, extension) = supported_upload_image_format(format)?;
    if actual_mime != expected_mime {
        return Err("上传图片 MIME 与真实图片格式不一致".to_string());
    }

    let decoded = image::load_from_memory_with_format(&bytes, format)
        .map_err(|err| format!("上传图片解析失败：{err}"))?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err("上传图片尺寸无效".to_string());
    }
    let file_size =
        i64::try_from(bytes.len()).map_err(|_| "上传图片过大，无法记录文件大小".to_string())?;
    let source_file_name = required_upload_text(file_name, "上传图片文件名")?;

    Ok(DecodedUploadedImage {
        bytes,
        extension,
        mime_type: actual_mime,
        width: width as i32,
        height: height as i32,
        file_size,
        source_file_name,
    })
}

fn normalize_upload_mime(mime_type: &str) -> Result<&'static str, String> {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/png" => Ok("image/png"),
        "image/jpeg" | "image/jpg" => Ok("image/jpeg"),
        "image/webp" => Ok("image/webp"),
        _ => Err("只支持 PNG/JPEG/WebP 图片".to_string()),
    }
}

fn strip_data_url_prefix(value: &str) -> &str {
    value
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(value)
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

fn required_upload_text(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name}不能为空"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::decode_uploaded_image_data;

    const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    #[test]
    fn uploaded_image_decode_accepts_base64_and_data_url() {
        for data in [
            PNG_1X1.to_string(),
            format!("data:image/png;base64,{PNG_1X1}"),
        ] {
            let decoded = decode_uploaded_image_data(" ref.png ", "image/png", &data, 1024 * 1024)
                .expect("valid uploaded png should decode");

            assert_eq!(decoded.source_file_name, "ref.png");
            assert_eq!(decoded.mime_type, "image/png");
            assert_eq!(decoded.extension, "png");
            assert_eq!((decoded.width, decoded.height), (1, 1));
            assert_eq!(decoded.file_size, decoded.bytes.len() as i64);
        }
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
