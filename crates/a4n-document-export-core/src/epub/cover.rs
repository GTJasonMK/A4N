use super::model::EpubMetadata;
use crate::DocumentExportError;

pub(super) struct CoverMetadata<'a> {
    pub data: &'a [u8],
    pub mime: &'a str,
    pub extension: &'static str,
}

pub(super) fn cover_metadata(
    metadata: &EpubMetadata,
) -> Result<Option<CoverMetadata<'_>>, DocumentExportError> {
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
        (Some(_), None) => Err(DocumentExportError::InvalidEpub(
            "EPUB 封面缺少 MIME 类型".into(),
        )),
        (None, Some(_)) => Err(DocumentExportError::InvalidEpub(
            "EPUB 封面缺少图片数据".into(),
        )),
    }
}

fn cover_extension(mime: &str) -> Result<&'static str, DocumentExportError> {
    match mime.trim() {
        "image/jpeg" => Ok("jpg"),
        "image/png" => Ok("png"),
        other => Err(DocumentExportError::InvalidEpub(format!(
            "EPUB 封面 MIME 类型不支持: {other}"
        ))),
    }
}
