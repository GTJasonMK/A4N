//! Provider-side model discovery shared by embedded mode and sidecars.
//!
//! This focused crate owns provider endpoint selection, authenticated model
//! list requests, response parsing, and secret-safe error projection. Host and
//! orchestrator layers retain configuration and secret ownership.

mod endpoints;
mod http;
mod parsing;

pub use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
pub use http::{discover_models, discover_models_from_request};

#[cfg(test)]
mod tests;
