use super::model::EpubMetadata;
use a4n_core::error::AppError;

pub(super) struct CoverMetadata<'a> {
    pub data: &'a [u8],
    pub mime: &'a str,
    pub extension: &'static str,
}

pub(super) fn cover_metadata(
    metadata: &EpubMetadata,
) -> Result<Option<CoverMetadata<'_>>, AppError> {
    match (&metadata.cover_image, &metadata.cover_mime) {
        (Some(data), Some(mime)) => {
            let mime = mime.trim();
            Ok(Some(CoverMetadata {
                data: data.as_slice(),
                mime,
                extension: cover_extension(mime)?,
            }))
        }
        (None, None) => Ok(None),
        (Some(_), None) => Err(AppError::InvalidArgument("EPUB 封面缺少 MIME 类型".into())),
        (None, Some(_)) => Err(AppError::InvalidArgument("EPUB 封面缺少图片数据".into())),
    }
}

fn cover_extension(mime: &str) -> Result<&'static str, AppError> {
    match mime.trim() {
        "image/jpeg" => Ok("jpg"),
        "image/png" => Ok("png"),
        other => Err(AppError::InvalidArgument(format!(
            "EPUB 封面 MIME 类型不支持: {other}"
        ))),
    }
}
