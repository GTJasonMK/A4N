pub use a4n_api_contract::embedding::{
    CancelEmbeddingModelDownloadInput, EmbeddingModelDownloadFileSummary,
    EmbeddingModelDownloadInfoSummary, EmbeddingModelInput, LocalEmbedTextsInput,
    LocalEmbedTextsOutput, LocalEmbeddingConfigInput, LocalEmbeddingModelSummary,
};

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingModelDownloadTaskScope {
    pub total: u32,
    pub initial_message: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EmbeddingModelExecution {
    Sidecar,
    HostLocal,
}
