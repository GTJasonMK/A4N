use std::path::{Path, PathBuf};

use fastembed::{EmbeddingModel, ModelInfo};
use hf_hub::api::tokio::{Api, ApiRepo};
use hf_hub::{Cache, Repo, RepoType};
use tokio_util::sync::CancellationToken;

use super::cache::cached_file_size;
use super::client::{build_hf_api, ensure_not_cancelled};
use super::{DOWNLOAD_CANCELLED_MESSAGE, FastEmbedDownloadFile};
use crate::proxy::ScopedHfHubEnv;
use crate::required_files;

pub(super) async fn collect_required_download_files_for_download(
    info: ModelInfo<EmbeddingModel>,
    cache_dir: PathBuf,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<FastEmbedDownloadFile>, String> {
    std::fs::create_dir_all(&cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    ensure_not_cancelled(cancel_token)?;
    let api = {
        let _hf_env = ScopedHfHubEnv::set(&cache_dir)?;
        build_hf_api(&cache_dir)?
    };
    let repo = api.model(info.model_code.clone());
    collect_required_download_files(&api, &repo, &info, &cache_dir, cancel_token).await
}

async fn collect_required_download_files(
    api: &Api,
    repo: &ApiRepo,
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: &Path,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<FastEmbedDownloadFile>, String> {
    let cache_repo = Cache::new(cache_dir.to_path_buf())
        .repo(Repo::new(info.model_code.clone(), RepoType::Model));
    let mut files = Vec::new();
    for path in required_files(info) {
        ensure_not_cancelled(cancel_token)?;
        let url = repo.url(&path);
        let metadata = match cancel_token {
            Some(token) => tokio::select! {
                _ = token.cancelled() => return Err(DOWNLOAD_CANCELLED_MESSAGE.to_string()),
                result = api.metadata(&url) => result,
            },
            None => api.metadata(&url).await,
        }
        .map_err(|err| format!("读取模型文件大小失败：{path}：{err}"))?;
        let size_bytes = metadata.size() as u64;
        let cached_bytes = cached_file_size(&cache_repo, &path, metadata.etag(), size_bytes);
        files.push(FastEmbedDownloadFile {
            url,
            path,
            size_bytes,
            downloaded_size_bytes: cached_bytes,
            downloaded: cached_bytes >= size_bytes,
            etag: metadata.etag().to_string(),
            commit_hash: metadata.commit_hash().to_string(),
        });
    }
    Ok(files)
}
