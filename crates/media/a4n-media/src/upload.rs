//! 上传媒体的纯解析与格式校验。
//!
//! 本模块不访问数据库、应用状态或文件系统；调用方负责持久化和权限。

use std::{io::Cursor, path::Path};

use a4n_domain::media::{
    DecodedUploadedAudio, DecodedUploadedImage, ImageUploadFormat, canonical_audio_mime,
    image_upload_format_from_extension, image_upload_format_from_mime,
};
use base64::Engine as _;
use image::GenericImageView;

pub const MAX_IMPORT_AUDIO_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ImageDimensionsError {
    #[error("图片 provider 返回的媒体格式无法识别：{0}")]
    Format(#[source] std::io::Error),
    #[error("图片 provider 返回的媒体尺寸无法读取：{0}")]
    Dimensions(#[source] image::ImageError),
    #[error("图片宽度超出支持范围")]
    WidthOverflow,
    #[error("图片高度超出支持范围")]
    HeightOverflow,
}

pub fn image_dimensions_from_bytes(bytes: &[u8]) -> Result<(i32, i32), ImageDimensionsError> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(ImageDimensionsError::Format)?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(ImageDimensionsError::Dimensions)?;
    Ok((
        i32::try_from(width).map_err(|_| ImageDimensionsError::WidthOverflow)?,
        i32::try_from(height).map_err(|_| ImageDimensionsError::HeightOverflow)?,
    ))
}

pub fn required_upload_file_name(file_name: &str, label: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("{label}不能包含路径"));
    }
    let name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label}无效"))?;
    if name != trimmed {
        return Err(format!("{label}不能包含路径"));
    }
    Ok(name.to_string())
}

pub fn decode_uploaded_bytes(
    data_base64: &str,
    label: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let raw = data_base64.trim();
    if raw.is_empty() {
        return Err(format!("{label}数据不能为空"));
    }
    let max_base64_len = max_bytes.div_ceil(3) * 4;
    if raw.len() > max_base64_len {
        return Err(format!("{label}不能超过 {}MB", max_bytes / 1024 / 1024));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|error| format!("{label} base64 解码失败：{error}"))?;
    if bytes.is_empty() {
        return Err(format!("{label}文件不能为空"));
    }
    if bytes.len() > max_bytes {
        return Err(format!("{label}不能超过 {}MB", max_bytes / 1024 / 1024));
    }
    Ok(bytes)
}

pub fn decode_uploaded_audio_data(
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
) -> Result<DecodedUploadedAudio, String> {
    let file_name = required_upload_file_name(file_name, "上传音频文件名")?;
    let (mime_type, extension) = canonical_audio_mime(mime_type)?;
    let actual_extension = Path::new(&file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "上传音频文件名缺少扩展名".to_string())?;
    if !actual_extension.eq_ignore_ascii_case(extension) {
        return Err(format!(
            "上传音频文件扩展名与 MIME 不一致，需要 .{extension} 文件"
        ));
    }
    let bytes = decode_uploaded_bytes(data_base64, "上传音频", MAX_IMPORT_AUDIO_BYTES)?;
    Ok(DecodedUploadedAudio {
        file_name,
        mime_type,
        extension,
        bytes,
    })
}

pub fn decode_uploaded_image_data(
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
    max_bytes: usize,
) -> Result<DecodedUploadedImage, String> {
    let expected_mime = normalize_upload_mime(mime_type)?;
    let bytes = decode_uploaded_bytes(data_base64, "上传图片", max_bytes)?;
    let format = image::guess_format(&bytes).map_err(|_| "上传图片不是有效图片".to_string())?;
    let image_format = image_upload_format_from_image_format(format)
        .ok_or_else(|| "上传图片格式不支持，仅支持 PNG/JPEG/WebP".to_string())?;
    let actual_mime = image_format.mime_type;
    let extension = image_format.extension;
    if actual_mime != expected_mime {
        return Err("上传图片 MIME 与真实图片格式不一致".to_string());
    }
    let decoded = image::load_from_memory_with_format(&bytes, format)
        .map_err(|error| format!("上传图片解析失败：{error}"))?;
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
    image_upload_format_from_mime(mime_type)
        .map(|format| format.mime_type)
        .ok_or_else(|| "只支持 PNG/JPEG/WebP 图片".to_string())
}

pub fn image_upload_format_from_image_format(
    format: image::ImageFormat,
) -> Option<&'static ImageUploadFormat> {
    format
        .extensions_str()
        .iter()
        .find_map(|extension| image_upload_format_from_extension(extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    #[test]
    fn supported_codecs_decode_uploads_and_provider_dimensions() {
        let image = image::DynamicImage::new_rgb8(3, 2);
        for (format, mime, extension, name) in [
            (image::ImageFormat::Png, "image/png", "png", "test.png"),
            (image::ImageFormat::Jpeg, "image/jpeg", "jpg", "test.jpg"),
            (image::ImageFormat::WebP, "image/webp", "webp", "test.webp"),
        ] {
            let mut bytes = Cursor::new(Vec::new());
            image.write_to(&mut bytes, format).unwrap();
            assert_eq!(
                image_dimensions_from_bytes(bytes.get_ref()).unwrap(),
                (3, 2)
            );
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes.get_ref());
            let decoded = decode_uploaded_image_data(name, mime, &encoded, 1024 * 1024).unwrap();
            assert_eq!((decoded.width, decoded.height), (3, 2));
            assert_eq!((decoded.mime_type, decoded.extension), (mime, extension));
        }
        assert!(image_upload_format_from_image_format(image::ImageFormat::Gif).is_none());
    }

    #[test]
    fn uploaded_audio_requires_plain_matching_file_name() {
        assert!(decode_uploaded_audio_data("dir/voice.mp3", "audio/mpeg", "SUQz").is_err());
        assert!(decode_uploaded_audio_data("voice.wav", "audio/mpeg", "SUQz").is_err());
        let source = decode_uploaded_audio_data(" voice.mp3 ", "audio/mp3", "SUQz").unwrap();
        assert_eq!(source.file_name, "voice.mp3");
        assert_eq!(source.mime_type, "audio/mpeg");
    }

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
    fn uploaded_image_decode_rejects_mime_format_mismatch() {
        let err = decode_uploaded_image_data("ref.jpg", "image/jpeg", PNG_1X1, 1024 * 1024)
            .expect_err("declared jpeg with png bytes should fail");
        assert_eq!(err, "上传图片 MIME 与真实图片格式不一致");
    }

    #[test]
    fn provider_image_dimensions_use_actual_media_bytes() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PNG_1X1)
            .expect("测试 PNG 应有效");
        assert_eq!(image_dimensions_from_bytes(&bytes).unwrap(), (1, 1));
    }
}
