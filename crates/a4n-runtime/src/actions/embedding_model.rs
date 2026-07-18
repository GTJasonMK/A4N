mod contracts;
mod download;
mod models;
mod provider;
mod rules;
mod runtime;

pub use a4n_embedding::provider::DownloadProgress;
pub use contracts::{
    CancelEmbeddingModelDownloadInput, EmbeddingModelDownloadFileSummary,
    EmbeddingModelDownloadInfoSummary, EmbeddingModelDownloadTaskScope, EmbeddingModelInput,
    LocalEmbedTextsInput, LocalEmbedTextsOutput, LocalEmbeddingConfigInput,
    LocalEmbeddingModelSummary,
};
pub use download::{download_embedding_model, download_embedding_model_to_cache};
pub use models::{
    delete_local_embedding_model, delete_local_embedding_model_in_cache,
    embedding_model_download_task_scope, get_embedding_model_download_info,
    get_embedding_model_download_info_in_cache, list_local_embedding_models,
    list_local_embedding_models_in_cache, required_task_id,
};
pub use runtime::LocalEmbeddingProviderRuntime;

#[cfg(test)]
mod tests;
