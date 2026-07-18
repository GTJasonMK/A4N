pub(in crate::download) const DOWNLOAD_CANCELLED_MESSAGE: &str = "模型下载已取消";

#[derive(Debug, Clone, serde::Serialize)]
pub struct FastEmbedDownloadInfo {
    pub model_name: String,
    pub model_code: String,
    pub total_size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub missing_size_bytes: u64,
    pub files: Vec<FastEmbedDownloadFile>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FastEmbedDownloadFile {
    pub path: String,
    pub size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub downloaded: bool,
    #[serde(skip)]
    pub(in crate::download) url: String,
    #[serde(skip)]
    pub(in crate::download) etag: String,
    #[serde(skip)]
    pub(in crate::download) commit_hash: String,
}
