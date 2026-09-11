//! Local document-format rendering.
//!
//! This crate is a Host-side format adapter. It deliberately does not own the
//! export capability wire contract or package/archive transport.

pub mod epub;
mod errors;
pub mod pdf;

pub use errors::DocumentExportError;
