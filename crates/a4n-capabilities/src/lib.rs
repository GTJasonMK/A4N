//! Host-side capability facade traits and HTTP clients.

mod contracts;
mod embedding;
mod llm;
mod media;
mod package_io;
mod prompt;
mod rag;
#[cfg(test)]
mod tests;
mod transport;

pub use a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY as EVENT_QUEUE_CAPACITY;

pub use contracts::{
    A4nCapabilities, CapabilityClient, CapabilityResult, EmbeddingClient, ExportClient,
    ImageClient, ImportClient, JsonRequest, JsonResponse, LlmClient, PromptClient, RagClient,
    VideoClient,
};
pub use embedding::HttpEmbeddingClient;
pub use llm::HttpLlmClient;
pub use media::{HttpImageClient, HttpVideoClient};
pub use package_io::{HttpExportClient, HttpImportClient};
pub use prompt::HttpPromptClient;
pub use rag::HttpRagClient;
pub use transport::HttpClientFactory;
