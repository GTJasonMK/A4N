use super::model::EpubMetadata;
use crate::ExportRenderError;

pub(super) struct CoverMetadata<'a> {
    pub data: &'a [u8],
    pub mime: &'a str,
    pub extension: &'static str,
}

pub(super) fn cover_metadata(
    metadata: &EpubMetadata,
) -> Result<Option<CoverMetadata<'_>>, ExportRenderError> {
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
        (Some(_), None) => Err(ExportRenderError::InvalidEpub(
            "EPUB 封面缺少 MIME 类型".into(),
        )),
        (None, Some(_)) => Err(ExportRenderError::InvalidEpub(
            "EPUB 封面缺少图片数据".into(),
        )),
    }
}

fn cover_extension(mime: &str) -> Result<&'static str, ExportRenderError> {
    match mime.trim() {
        "image/jpeg" => Ok("jpg"),
        "image/png" => Ok("png"),
        other => Err(ExportRenderError::InvalidEpub(format!(
            "EPUB 封面 MIME 类型不支持: {other}"
        ))),
    }
}
