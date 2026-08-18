use std::path::PathBuf;

use a4n_core::error::AppError;
use a4n_domain::embedding::EmbeddingModelDownloadInfoSummary;

use super::contracts::download_byte_totals;
use super::metadata::collect_required_download_files;
use crate::models::resolve_model_info;

pub async fn fastembed_model_download_info(
    model_name: &str,
    cache_dir: PathBuf,
) -> Result<EmbeddingModelDownloadInfoSummary, AppError> {
    let resolved = resolve_model_info(model_name)?;
    let files = collect_required_download_files(&resolved.info, cache_dir)
        .await
        .map_err(AppError::ConfigError)?;
    let (total_size_bytes, downloaded_size_bytes) =
        download_byte_totals(&files).map_err(AppError::ConfigError)?;
    if total_size_bytes == 0 {
        return Err(AppError::ConfigError(format!(
            "fastembed 模型下载信息未包含有效文件大小：{}（{}）",
            resolved.name, resolved.info.model_code
        )));
    }
    let missing_size_bytes = total_size_bytes - downloaded_size_bytes;

    Ok(EmbeddingModelDownloadInfoSummary {
        model_name: resolved.name,
        model_code: resolved.info.model_code,
        total_size_bytes,
        downloaded_size_bytes,
        missing_size_bytes,
    })
}
