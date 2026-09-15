//! Concrete HTTP and deterministic local embedding providers.
//!
//! Provider contracts stay in `a4n-embedding-provider-contract`; this crate
//! owns only executable provider implementations.

pub mod http;
pub mod local;

pub use http::HttpEmbeddingProvider;
pub use local::LocalEmbeddingProvider;
