mod embedding;
mod export;
mod image;
mod llm;
mod media;
mod rag;
mod video;

pub use embedding::{
    PendingEmbeddingDownloadClient, PendingEmbeddingDownloadObserver, RecordingEmbeddingClient,
};
pub use export::RecordingExportClient;
pub use image::RecordingImageClient;
pub use llm::RecordingLlmClient;
pub use rag::RecordingRagClient;
pub use video::RecordingVideoClient;

use a4n_service_protocol::{ServiceError, ServiceErrorCode};

fn unexpected_call(client: &str, operation: &str) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::Internal,
        message: format!("unexpected {client} {operation} call"),
        retryable: Some(false),
    }
}

#[cfg(test)]
mod tests;
