mod download;
mod host_adapter;
mod models;
mod provider;
mod runtime;

pub use a4n_api_contract::embedding::{
    CancelEmbeddingModelDownloadInput, DownloadEmbeddingModelInput,
    EmbeddingModelDownloadInfoSummary, EmbeddingModelInput, LocalEmbedTextsInput,
    LocalEmbedTextsOutput, LocalEmbeddingConfigInput, LocalEmbeddingModelSummary,
};
pub use a4n_embedding::provider::DownloadProgress;
pub use download::{download_embedding_model, download_embedding_model_to_cache};
pub use models::{
    delete_local_embedding_model, delete_local_embedding_model_in_cache,
    get_embedding_model_download_info, get_embedding_model_download_info_in_cache,
    list_local_embedding_models, list_local_embedding_models_in_cache,
};
pub use runtime::LocalEmbeddingProviderRuntime;

pub const EMBEDDING_MODEL_DOWNLOAD_TASK_TOTAL: u32 = 1000;
pub const EMBEDDING_MODEL_DOWNLOAD_INITIAL_MESSAGE: &str = "正在读取模型文件大小";

#[cfg(test)]
mod tests;
