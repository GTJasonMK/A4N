use std::io::{Seek, SeekFrom, Write};

use reqwest::Client;
use reqwest::header::RANGE;
use tokio_util::sync::CancellationToken;

use super::cache::{create_snapshot_pointer, file_size};
use super::client::{cancelled_or_error, ensure_not_cancelled};
use super::progress::{AggregatedDownloadProgress, format_bytes};
use super::{DOWNLOAD_CANCELLED_MESSAGE, FastEmbedDownloadFile};

pub(super) async fn download_required_file(
    client: &Client,
    cache_repo: &hf_hub::CacheRepo,
    file: &FastEmbedDownloadFile,
    progress: &mut AggregatedDownloadProgress,
    cancel_token: Option<&CancellationToken>,
) -> Result<(), String> {
    let blob_path = cache_repo.blob_path(&file.etag);
    std::fs::create_dir_all(blob_path.parent().ok_or_else(|| {
        format!(
            "模型缓存 blob 路径缺少父目录：{}",
            blob_path.to_string_lossy()
        )
    })?)
    .map_err(|err| format!("创建模型缓存 blob 目录失败：{err}"))?;

    if file_size(&blob_path) >= file.size_bytes {
        create_snapshot_pointer(cache_repo, &blob_path, &file.commit_hash, &file.path)?;
        return Ok(());
    }

    let mut part_path = blob_path.clone();
    part_path.set_extension("part");
    let mut part = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(&part_path)
        .map_err(|err| format!("打开临时下载文件失败：{err}"))?;
    let mut start = part
        .metadata()
        .map_err(|err| format!("读取临时下载文件状态失败：{err}"))?
        .len();
    if start > file.size_bytes {
        part.set_len(0)
            .map_err(|err| format!("重置异常临时下载文件失败：{err}"))?;
        start = 0;
    }
    part.seek(SeekFrom::End(0))
        .map_err(|err| format!("定位临时下载文件失败：{err}"))?;

    progress.emit(true);
    let mut request = client.get(&file.url);
    if start > 0 {
        request = request.header(RANGE, format!("bytes={start}-"));
    }
    ensure_not_cancelled(cancel_token)?;
    let response_result = match cancel_token {
        Some(token) => {
            tokio::select! {
                _ = token.cancelled() => return Err(DOWNLOAD_CANCELLED_MESSAGE.to_string()),
                result = request.send() => result,
            }
        }
        None => request.send().await,
    };
    let response = response_result
        .map_err(|err| cancelled_or_error(cancel_token, || format!("请求模型文件失败：{err}")))?;
    if start > 0 && response.status() == reqwest::StatusCode::OK {
        return Err("服务器未接受断点续传请求，请删除该模型缓存后重新下载".to_string());
    }
    let mut response = response
        .error_for_status()
        .map_err(|err| cancelled_or_error(cancel_token, || format!("模型文件响应失败：{err}")))?;

    loop {
        ensure_not_cancelled(cancel_token)?;
        let chunk_result = match cancel_token {
            Some(token) => {
                tokio::select! {
                    _ = token.cancelled() => return Err(DOWNLOAD_CANCELLED_MESSAGE.to_string()),
                    result = response.chunk() => result,
                }
            }
            None => response.chunk().await,
        };
        let Some(chunk) = chunk_result.map_err(|err| {
            cancelled_or_error(cancel_token, || format!("读取模型文件响应失败：{err}"))
        })?
        else {
            break;
        };
        part.write_all(&chunk)
            .map_err(|err| format!("写入临时下载文件失败：{err}"))?;
        let chunk_size = u64::try_from(chunk.len())
            .map_err(|_| format!("模型下载分块大小超过 u64 上限：{}", chunk.len()))?;
        progress.update(chunk_size)?;
    }
    part.flush()
        .map_err(|err| format!("刷新临时下载文件失败：{err}"))?;
    drop(part);

    let final_size = file_size(&part_path);
    if final_size != file.size_bytes {
        return Err(format!(
            "模型文件大小不完整：{} / {}",
            format_bytes(final_size),
            format_bytes(file.size_bytes)
        ));
    }

    std::fs::rename(&part_path, &blob_path)
        .map_err(|err| format!("提交模型缓存文件失败：{err}"))?;
    create_snapshot_pointer(cache_repo, &blob_path, &file.commit_hash, &file.path)?;
    progress.emit(true);
    Ok(())
}
