#[derive(Debug, thiserror::Error)]
pub enum ExportRenderError {
    #[error("导出 JSON 序列化失败：{0}")]
    Json(#[from] serde_json::Error),
    #[error("{label}序列化失败：{source}")]
    JsonManifest {
        label: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{0}")]
    InvalidZipEntryPath(#[from] a4n_archive_core::ArchivePathError),
    #[error("{label} base64 解码失败：{source}")]
    Base64Decode {
        label: String,
        #[source]
        source: base64::DecodeError,
    },
    #[error("写入 zip 条目失败：{path}: {source}")]
    ZipStartFile {
        path: String,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("导出 zip 条目重复：{path}")]
    DuplicateZipEntry { path: String },
    #[error("写入{label}失败：{source}")]
    ZipEntryWrite {
        label: String,
        #[source]
        source: std::io::Error,
    },
    #[error("完成{label}失败：{source}")]
    ZipFinish {
        label: String,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("参数无效：{0}")]
    InvalidEpub(String),
    #[error("内部错误：EPUB 写入失败: {source}")]
    EpubZipStart {
        #[source]
        source: zip::result::ZipError,
    },
    #[error("内部错误：EPUB 写入失败: {source}")]
    EpubEntryWrite {
        #[source]
        source: std::io::Error,
    },
    #[error("内部错误：EPUB 打包失败: {source}")]
    EpubFinish {
        #[source]
        source: zip::result::ZipError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epub_write_errors_keep_app_error_display_prefix() {
        let start = ExportRenderError::EpubZipStart {
            source: zip::result::ZipError::FileNotFound,
        };
        let write = ExportRenderError::EpubEntryWrite {
            source: std::io::Error::other("disk full"),
        };
        let finish = ExportRenderError::EpubFinish {
            source: zip::result::ZipError::FileNotFound,
        };

        assert!(start.to_string().starts_with("内部错误：EPUB 写入失败: "));
        assert_eq!(write.to_string(), "内部错误：EPUB 写入失败: disk full");
        assert!(finish.to_string().starts_with("内部错误：EPUB 打包失败: "));
    }
}
