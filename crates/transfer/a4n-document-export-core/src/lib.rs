//! Local document-format rendering.
//!
//! This crate owns pure document-format rendering. It uses the export data
//! contract but deliberately does not own service transport or authorization.

mod document;
pub mod epub;
mod errors;
pub mod pdf;

pub use document::{
    chapter_heading, render_markdown_document, render_pdf_document, render_txt_document,
};
pub use errors::DocumentExportError;
