//! Pure export rendering contracts.
//!
//! This crate renders already-authorized, already-materialized payloads. It
//! does not read the application database and does not decide what a user may
//! export.

mod archive;
mod contracts;
pub mod epub;
mod errors;
mod json;
pub mod pdf;

pub use archive::{render_zip, render_zip_package};
pub use contracts::*;
pub use errors::ExportRenderError;
pub use json::{render_json, render_json_manifest_pretty};

#[cfg(test)]
mod tests;
