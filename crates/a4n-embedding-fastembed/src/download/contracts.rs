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

pub(in crate::download) fn download_byte_totals(
    files: &[FastEmbedDownloadFile],
) -> Result<(u64, u64), String> {
    let mut total = 0_u64;
    let mut downloaded = 0_u64;
    for file in files {
        total = total.checked_add(file.size_bytes).ok_or_else(|| {
            format!(
                "fastembed 模型文件总大小超过 u64 上限：{total} + {}",
                file.size_bytes
            )
        })?;
        downloaded = downloaded
            .checked_add(file.downloaded_size_bytes)
            .ok_or_else(|| {
                format!(
                    "fastembed 已下载文件总大小超过 u64 上限：{downloaded} + {}",
                    file.downloaded_size_bytes
                )
            })?;
    }
    if downloaded > total {
        return Err(format!(
            "fastembed 下载状态不一致：已下载 {downloaded} 字节大于模型总大小 {total} 字节"
        ));
    }
    Ok((total, downloaded))
}
