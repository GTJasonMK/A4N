//! Lightweight embedding contracts and provider implementations.
//!
//! Heavy local model support lives in `a4n-embedding-fastembed`; this crate is
//! kept small enough for host/orchestrator code to depend on without pulling
//! HuggingFace or ONNX runtime dependencies.

pub mod http;
pub mod local;
pub mod provider;
