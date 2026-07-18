//! fastembed 本地嵌入模型后端。
//!
//! 这里刻意把“下载模型”和“使用模型”拆开：只有显式下载 API 会触发
//! HuggingFace 拉取；RAG 入库/检索时若模型未缓存完整，会直接失败并提示先下载。

mod cache;
mod download;
mod models;
mod provider;
mod proxy;

use self::cache::required_files;

pub use self::download::{
    FastEmbedDownloadFile, FastEmbedDownloadInfo, download_fastembed_model,
    fastembed_model_download_info,
};
pub use self::models::{
    DEFAULT_FASTEMBED_MODEL, FastEmbedLocalModel, delete_fastembed_model,
    fastembed_model_dimension, list_fastembed_local_models,
};
pub use self::provider::FastEmbedProvider;

#[cfg(test)]
mod tests;
