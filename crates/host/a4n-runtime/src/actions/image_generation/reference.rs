use a4n_domain::media::image_mime_from_path;
use a4n_error::AppError;
use a4n_image_contract::ImageReference;
use a4n_storage::canonicalize_data_file_path;
use base64::Engine as _;
use image::GenericImageView;
use std::io::Cursor;
use std::path::Path;

const MAX_REFERENCE_IMAGE_SIDE: u32 = 512;
const MAX_REFERENCE_IMAGE_BYTES: usize = 700 * 1024;
const REFERENCE_JPEG_QUALITY: u8 = 88;

pub(crate) fn read_reference_image(
    data_dir: &Path,
    label: &str,
    relative_path: &str,
) -> Result<ImageReference, AppError> {
    let path = canonicalize_data_file_path(data_dir, relative_path, &format!("参考图 {label}"))
        .map_err(host_error)?;
    let bytes = std::fs::read(&path)
        .map_err(|error| host_error(format!("读取参考图 {} 失败：{error}", path.display())))?;
    let mime = image_mime_from_path(&path).map_err(host_error)?;
    let (mime, bytes) = compact_reference_image(&bytes, mime, label)?;
    Ok(ImageReference {
        label: label.to_string(),
        data_uri: format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ),
    })
}

fn compact_reference_image(
    bytes: &[u8],
    mime: &str,
    label: &str,
) -> Result<(&'static str, Vec<u8>), AppError> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| host_error(format!("参考图 {label} 格式无法识别：{error}")))?;
    let format = reader
        .format()
        .ok_or_else(|| host_error(format!("参考图 {label} 格式无法识别")))?;
    let actual_mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => "image/webp",
        _ => return Err(host_error(format!("参考图 {label} 格式不支持"))),
    };
    if actual_mime != mime {
        return Err(host_error(format!(
            "参考图 {label} MIME 类型 {mime} 与实际格式 {actual_mime} 不一致"
        )));
    }
    let image = reader
        .decode()
        .map_err(|error| host_error(format!("参考图 {label} 解码失败：{error}")))?;
    let (width, height) = image.dimensions();
    if bytes.len() <= MAX_REFERENCE_IMAGE_BYTES
        && width <= MAX_REFERENCE_IMAGE_SIDE
        && height <= MAX_REFERENCE_IMAGE_SIDE
    {
        return Ok((actual_mime, bytes.to_vec()));
    }

    let resized = image.thumbnail(MAX_REFERENCE_IMAGE_SIDE, MAX_REFERENCE_IMAGE_SIDE);
    let mut compacted = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut compacted, REFERENCE_JPEG_QUALITY)
        .encode_image(&image::DynamicImage::ImageRgb8(resized.to_rgb8()))
        .map_err(|error| host_error(format!("参考图 {label} 压缩失败：{error}")))?;
    Ok(("image/jpeg", compacted))
}

fn host_error(message: String) -> AppError {
    AppError::InvalidArgument(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_reference_images_are_compacted_before_sidecar_serialization() {
        let temp = tempfile::tempdir().expect("临时目录应创建");
        let path = temp.path().join("reference.png");
        let image = image::RgbImage::from_pixel(2048, 1024, image::Rgb([20, 40, 60]));
        image::DynamicImage::ImageRgb8(image)
            .save(&path)
            .expect("测试图片应写入");

        let reference =
            read_reference_image(temp.path(), "角色", "reference.png").expect("参考图应成功压缩");
        assert!(reference.data_uri.starts_with("data:image/jpeg;base64,"));
        let encoded = reference
            .data_uri
            .split_once(',')
            .expect("data URI 应有正文")
            .1;
        let compacted = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("压缩后的 base64 应有效");
        let decoded = image::load_from_memory(&compacted).expect("压缩后的图片应可解码");
        assert!(decoded.width() <= MAX_REFERENCE_IMAGE_SIDE);
        assert!(decoded.height() <= MAX_REFERENCE_IMAGE_SIDE);
        assert!(compacted.len() <= MAX_REFERENCE_IMAGE_BYTES);
    }

    #[test]
    fn small_reference_images_keep_their_original_format() {
        let temp = tempfile::tempdir().expect("临时目录应创建");
        let path = temp.path().join("reference.png");
        image::DynamicImage::new_rgb8(2, 2)
            .save(&path)
            .expect("测试图片应写入");

        let reference =
            read_reference_image(temp.path(), "角色", "reference.png").expect("小参考图应成功读取");
        assert!(reference.data_uri.starts_with("data:image/png;base64,"));
    }
}
