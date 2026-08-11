//! Pure import transformation contracts.
//!
//! This crate intentionally does not know about projects, permissions, IDs,
//! repositories, or the application database. Host/orchestrator code owns those
//! business concerns.

mod archive;
mod contracts;
pub mod dedup;
mod errors;
pub mod file_parsers;
mod json;
mod media_package;

pub use archive::{
    read_json_manifest_entry, read_zip_entries_bytes, read_zip_entry_bytes,
    validate_zip_declared_files, validate_zip_entry_path, validate_zip_manifest_declared_files,
};
pub use contracts::*;
pub use errors::ImportTransformError;
pub use json::{parse_json_manifest, parse_json_manifest_value};
pub use media_package::{
    read_media_package_zip, read_media_package_zip_bytes, read_media_package_zip_reader,
};

#[cfg(test)]
mod tests;
