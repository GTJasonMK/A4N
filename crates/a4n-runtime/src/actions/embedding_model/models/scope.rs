use a4n_core::error::AppError;

use super::super::{CancelEmbeddingModelDownloadInput, host_adapter::required_text};

pub const EMBEDDING_MODEL_DOWNLOAD_TASK_TOTAL: u32 = 1000;
pub const EMBEDDING_MODEL_DOWNLOAD_INITIAL_MESSAGE: &str = "正在读取模型文件大小";

pub fn required_task_id(input: CancelEmbeddingModelDownloadInput) -> Result<String, AppError> {
    normalize_embedding_download_task_id(&input.task_id)
}

pub fn normalize_optional_embedding_download_task_id(
    task_id: Option<String>,
) -> Result<Option<String>, AppError> {
    task_id
        .as_deref()
        .map(normalize_embedding_download_task_id)
        .transpose()
}

fn normalize_embedding_download_task_id(task_id: &str) -> Result<String, AppError> {
    let task_id = required_text(task_id, "模型下载任务 ID")?;
    if task_id.len() > 160 {
        return Err(AppError::InvalidArgument(
            "模型下载任务 ID 长度不能超过 160 个字符".to_string(),
        ));
    }
    if !task_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err(AppError::InvalidArgument(
            "模型下载任务 ID 只能包含 ASCII 字母、数字、-、_、:、.".to_string(),
        ));
    }
    Ok(task_id)
}
