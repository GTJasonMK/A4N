use std::io::{Seek, Write};

use crate::ExportRenderError;

pub(super) fn write_entry<W: Write + Seek>(
    writer: &mut zip::ZipWriter<W>,
    path: &str,
    data: &[u8],
    options: zip::write::SimpleFileOptions,
) -> Result<(), ExportRenderError> {
    writer
        .start_file(path, options)
        .map_err(|source| ExportRenderError::EpubZipStart { source })?;
    writer
        .write_all(data)
        .map_err(|source| ExportRenderError::EpubEntryWrite { source })
}
