use std::collections::HashSet;
use std::io::{Cursor, Seek, Write};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::contracts::{ExportRenderZipInput, ExportRenderZipOutput, ExportZipPackageInput};
use crate::errors::ExportRenderError;

pub fn render_zip(input: ExportRenderZipInput) -> Result<ExportRenderZipOutput, ExportRenderError> {
    let bytes = render_zip_package(input.into_package_input()?)?;
    Ok(ExportRenderZipOutput {
        bytes_base64: BASE64.encode(bytes),
        content_type: "application/zip".to_string(),
    })
}

pub fn render_zip_package(input: ExportZipPackageInput) -> Result<Vec<u8>, ExportRenderError> {
    let mut zip = ExportZipWriter::new(Cursor::new(Vec::new()));
    zip.write_entry_bytes(
        &input.manifest_path,
        &input.manifest_bytes,
        input.manifest_label,
    )?;
    for entry in input.entries {
        zip.write_entry_bytes(&entry.path, &entry.bytes, entry.label)?;
    }
    let cursor = zip.finish(input.package_label)?;
    Ok(cursor.into_inner())
}

pub(crate) fn decode_base64(
    label: impl Into<String>,
    value: &str,
) -> Result<Vec<u8>, ExportRenderError> {
    BASE64
        .decode(value)
        .map_err(|source| ExportRenderError::Base64Decode {
            label: label.into(),
            source,
        })
}

pub(crate) struct ExportZipWriter<W: Write + Seek> {
    zip: ZipWriter<W>,
    options: SimpleFileOptions,
    entries: HashSet<String>,
}

impl<W: Write + Seek> ExportZipWriter<W> {
    pub(crate) fn new(writer: W) -> Self {
        Self {
            zip: ZipWriter::new(writer),
            options: SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
            entries: HashSet::new(),
        }
    }

    pub(crate) fn write_entry_bytes(
        &mut self,
        path: &str,
        bytes: &[u8],
        label: impl Into<String>,
    ) -> Result<(), ExportRenderError> {
        let label = label.into();
        a4n_archive_core::validate_zip_entry_path(path)?;
        if !self.entries.insert(path.to_string()) {
            return Err(ExportRenderError::DuplicateZipEntry {
                path: path.to_string(),
            });
        }
        self.zip.start_file(path, self.options).map_err(|source| {
            ExportRenderError::ZipStartFile {
                path: path.to_string(),
                source,
            }
        })?;
        self.zip
            .write_all(bytes)
            .map_err(|source| ExportRenderError::ZipEntryWrite { label, source })
    }

    pub(crate) fn finish(self, label: impl Into<String>) -> Result<W, ExportRenderError> {
        self.zip
            .finish()
            .map_err(|source| ExportRenderError::ZipFinish {
                label: label.into(),
                source,
            })
    }
}
