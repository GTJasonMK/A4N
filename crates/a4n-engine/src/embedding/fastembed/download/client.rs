use std::path::Path;
use std::time::Duration;

use hf_hub::api::sync::{Api, ApiBuilder};
use reqwest::Client;
use tokio_util::sync::CancellationToken;

use super::DOWNLOAD_CANCELLED_MESSAGE;
use crate::embedding::fastembed::proxy::ScopedHfHubEnv;

const DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn build_download_client(cache_dir: &Path) -> Result<Client, String> {
    let _hf_env = ScopedHfHubEnv::set(cache_dir)?;
    Client::builder()
        .connect_timeout(DOWNLOAD_CONNECT_TIMEOUT)
        .build()
        .map_err(|err| format!("初始化模型下载 HTTP 客户端失败：{err}"))
}

pub(super) fn ensure_not_cancelled(cancel_token: Option<&CancellationToken>) -> Result<(), String> {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        Err(DOWNLOAD_CANCELLED_MESSAGE.to_string())
    } else {
        Ok(())
    }
}

pub(super) fn cancelled_or_error(
    cancel_token: Option<&CancellationToken>,
    error: impl FnOnce() -> String,
) -> String {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        DOWNLOAD_CANCELLED_MESSAGE.to_string()
    } else {
        error()
    }
}

pub(super) fn build_hf_api(cache_dir: &Path) -> Result<Api, String> {
    let endpoint =
        std::env::var("HF_ENDPOINT").unwrap_or_else(|_| "https://huggingface.co".to_string());
    ApiBuilder::new()
        .with_cache_dir(cache_dir.to_path_buf())
        .with_endpoint(endpoint)
        .with_progress(false)
        .build()
        .map_err(|err| format!("初始化 HuggingFace 下载客户端失败：{err}"))
}
