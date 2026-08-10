use std::path::PathBuf;

use fastembed::{EmbeddingModel, ModelInfo};

use a4n_core::error::AppError;

use super::contracts::FastEmbedDownloadInfo;
use super::metadata::collect_required_download_files_for_download;
use crate::models::resolve_model_info;

pub async fn fastembed_model_download_info(
    model_name: &str,
    cache_dir: PathBuf,
) -> Result<FastEmbedDownloadInfo, AppError> {
    let resolved = resolve_model_info(model_name)?;
    let model_name = resolved.name.clone();
    let model_code = resolved.info.model_code.clone();
    collect_fastembed_download_info(&model_name, &model_code, &resolved.info, cache_dir)
        .await
        .map_err(AppError::ConfigError)
}

async fn collect_fastembed_download_info(
    model_name: &str,
    model_code: &str,
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: PathBuf,
) -> Result<FastEmbedDownloadInfo, String> {
    let files = collect_required_download_files_for_download(info.clone(), cache_dir, None).await?;
    let total_size_bytes = files.iter().try_fold(0_u64, |total, file| {
        total.checked_add(file.size_bytes).ok_or_else(|| {
            format!(
                "fastembed 模型文件总大小超过 u64 上限：{total} + {}",
                file.size_bytes
            )
        })
    })?;
    if total_size_bytes == 0 {
        return Err(format!(
            "fastembed 模型下载信息未包含有效文件大小：{model_name}（{model_code}）"
        ));
    }
    let downloaded_size_bytes = files.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.downloaded_size_bytes)
            .ok_or_else(|| {
                format!(
                    "fastembed 已下载文件总大小超过 u64 上限：{total} + {}",
                    file.downloaded_size_bytes
                )
            })
    })?;
    let missing_size_bytes = total_size_bytes.checked_sub(downloaded_size_bytes).ok_or_else(|| {
        format!(
            "fastembed 下载状态不一致：已下载 {downloaded_size_bytes} 字节大于模型总大小 {total_size_bytes} 字节"
        )
    })?;

    Ok(FastEmbedDownloadInfo {
        model_name: model_name.to_string(),
        model_code: model_code.to_string(),
        total_size_bytes,
        downloaded_size_bytes,
        missing_size_bytes,
    })
}
