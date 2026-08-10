pub(in crate::download) const DOWNLOAD_CANCELLED_MESSAGE: &str = "模型下载已取消";

#[derive(serde::Serialize)]
pub struct FastEmbedDownloadInfo {
    pub model_name: String,
    pub model_code: String,
    pub total_size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub missing_size_bytes: u64,
}

pub(in crate::download) struct FastEmbedDownloadFile {
    pub(in crate::download) path: String,
    pub(in crate::download) size_bytes: u64,
    pub(in crate::download) downloaded_size_bytes: u64,
    pub(in crate::download) downloaded: bool,
    pub(in crate::download) url: String,
    pub(in crate::download) etag: String,
    pub(in crate::download) commit_hash: String,
}
