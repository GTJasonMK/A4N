use a4n_core::error::AppError;

use super::super::{
    contracts::{CancelEmbeddingModelDownloadInput, EmbeddingModelDownloadTaskScope},
    rules::required_text,
};

pub fn embedding_model_download_task_scope() -> EmbeddingModelDownloadTaskScope {
    EmbeddingModelDownloadTaskScope {
        total: 1000,
        initial_message: "正在读取模型文件大小",
    }
}

pub fn required_task_id(input: CancelEmbeddingModelDownloadInput) -> Result<String, AppError> {
    required_text(&input.task_id, "模型下载任务 ID")
}
