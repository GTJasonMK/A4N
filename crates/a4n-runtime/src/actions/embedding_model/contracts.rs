use a4n_embedding::provider::EmbeddingModelIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingModelSummary {
    pub name: String,
    pub model_code: String,
    pub description: String,
    pub dimension: usize,
    pub downloaded: bool,
    pub cache_path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelDownloadInfoSummary {
    pub model_name: String,
    pub model_code: String,
    pub total_size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub missing_size_bytes: u64,
    pub files: Vec<EmbeddingModelDownloadFileSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelDownloadFileSummary {
    pub path: String,
    pub size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingModelInput {
    pub model: String,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelEmbeddingModelDownloadInput {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingConfigInput {
    pub backend: String,
    pub model: String,
    pub dimension: usize,
    pub batch_size: Option<usize>,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalEmbedTextsInput {
    pub config: LocalEmbeddingConfigInput,
    pub texts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalEmbedTextsOutput {
    pub identity: EmbeddingModelIdentity,
    pub embeddings: Vec<Vec<f32>>,
}

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
