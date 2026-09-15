//! Compatibility facade for embedding provider implementations.
//!
//! New consumers should depend on `a4n-embedding-provider-contract` for types
//! and `a4n-embedding-providers` for concrete HTTP/local providers. Heavy local
//! model support lives in `a4n-embedding-fastembed`.

pub mod http;
pub mod local;
pub mod provider;
