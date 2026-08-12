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
    EMBEDDING_MODEL_DOWNLOAD_INITIAL_MESSAGE, EMBEDDING_MODEL_DOWNLOAD_TASK_TOTAL,
    delete_local_embedding_model, delete_local_embedding_model_in_cache,
    get_embedding_model_download_info, get_embedding_model_download_info_in_cache,
    list_local_embedding_models, list_local_embedding_models_in_cache,
    normalize_optional_embedding_download_task_id, required_task_id,
};
pub use runtime::LocalEmbeddingProviderRuntime;

#[cfg(test)]
mod tests;
