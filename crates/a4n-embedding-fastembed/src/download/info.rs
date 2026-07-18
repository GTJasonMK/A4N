use std::path::{Path, PathBuf};

use fastembed::{EmbeddingModel, ModelInfo};

use a4n_core::error::AppError;

use super::client::build_hf_api;
use super::contracts::FastEmbedDownloadInfo;
use super::metadata::collect_required_download_files;
use crate::models::resolve_model_info;
use crate::proxy::ScopedHfHubEnv;

pub async fn fastembed_model_download_info(
    model_name: &str,
    cache_dir: PathBuf,
) -> Result<FastEmbedDownloadInfo, AppError> {
    let resolved = resolve_model_info(model_name)?;
    let model_name = resolved.name.clone();
    let model_code = resolved.info.model_code.clone();
    tokio::task::spawn_blocking(move || {
        collect_fastembed_download_info(&model_name, &model_code, &resolved.info, &cache_dir)
    })
    .await
    .map_err(|err| AppError::EmbeddingError(format!("读取 fastembed 下载信息线程失败: {err}")))?
    .map_err(AppError::ConfigError)
}

fn collect_fastembed_download_info(
    model_name: &str,
    model_code: &str,
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: &Path,
) -> Result<FastEmbedDownloadInfo, String> {
    std::fs::create_dir_all(cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    let _hf_env = ScopedHfHubEnv::set(cache_dir)?;
    let api = build_hf_api(cache_dir)?;
    let repo = api.model(model_code.to_string());
    let files = collect_required_download_files(&api, &repo, info, cache_dir)?;
    let total_size_bytes = files.iter().try_fold(0_u64, |total, file| {
        total.checked_add(file.size_bytes).ok_or_else(|| {
            format!(
                "fastembed 模型文件总大小超过 u64 上限：{total} + {}",
                file.size_bytes
            )
        })
    })?;
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
        files,
    })
}
