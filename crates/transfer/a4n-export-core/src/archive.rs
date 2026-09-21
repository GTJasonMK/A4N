use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::errors::ExportRenderError;
use a4n_export_contract::{ExportRenderZipInput, ExportRenderZipOutput, ExportZipPackageInput};

pub fn render_zip(input: ExportRenderZipInput) -> Result<ExportRenderZipOutput, ExportRenderError> {
    let bytes = render_zip_package(input.into_package_input()?)?;
    Ok(ExportRenderZipOutput {
        bytes_base64: BASE64.encode(bytes),
        content_type: "application/zip".to_string(),
    })
}

pub fn render_zip_package(input: ExportZipPackageInput) -> Result<Vec<u8>, ExportRenderError> {
    let entries = std::iter::once(a4n_archive_core::ArchiveEntry {
        path: input.manifest_path,
        bytes: input.manifest_bytes,
        label: input.manifest_label,
    })
    .chain(
        input
            .entries
            .into_iter()
            .map(|entry| a4n_archive_core::ArchiveEntry {
                path: entry.path,
                bytes: entry.bytes,
                label: entry.label,
            }),
    );
    a4n_archive_core::write_zip(entries, input.package_label).map_err(Into::into)
}
