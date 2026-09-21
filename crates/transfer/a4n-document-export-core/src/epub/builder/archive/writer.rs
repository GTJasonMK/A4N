use std::io::{Seek, Write};

use crate::DocumentExportError;

pub(super) fn write_entry<W: Write + Seek>(
    writer: &mut zip::ZipWriter<W>,
    path: &str,
    data: &[u8],
    options: zip::write::SimpleFileOptions,
) -> Result<(), DocumentExportError> {
    writer
        .start_file(path, options)
        .map_err(|source| DocumentExportError::EpubZipStart { source })?;
    writer
        .write_all(data)
        .map_err(|source| DocumentExportError::EpubEntryWrite { source })
}
