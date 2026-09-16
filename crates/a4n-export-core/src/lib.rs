//! Pure export rendering contracts.
//!
//! Export Worker implementation for already-authorized, already-materialized
//! payloads. It does not read the application database or authorize exports.

mod archive;
mod document;
mod errors;
mod json;

pub use a4n_document_export_core::{
    chapter_heading, render_markdown_document, render_pdf_document, render_txt_document,
};
pub use a4n_export_contract::{ExportChapter, ExportDocument};
pub use archive::{render_zip, render_zip_package};
pub use document::render_document;
pub use errors::ExportRenderError;
pub use json::{render_json, render_json_manifest_pretty};

#[cfg(test)]
mod tests;
