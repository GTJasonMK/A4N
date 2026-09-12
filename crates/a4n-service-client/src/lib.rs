//! Generic HTTP transport for A4N sidecar services.

mod transport;

use async_trait::async_trait;

#[async_trait]
pub trait ServiceClient: Send + Sync {
    async fn health(&self) -> ServiceClientResult<a4n_service_protocol::ServiceHealth>;
}

pub use transport::{
    HttpClientFactory, HttpSidecarTransport, ServiceClientConfig, ServiceClientResult,
    sidecar_response_payload,
};

#[cfg(test)]
mod tests;
