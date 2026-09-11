#[derive(Debug, thiserror::Error)]
pub enum DocumentExportError {
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
    use super::DocumentExportError;

    #[test]
    fn epub_write_errors_keep_app_error_display_prefix() {
        let start = DocumentExportError::EpubZipStart {
            source: zip::result::ZipError::FileNotFound,
        };
        let write = DocumentExportError::EpubEntryWrite {
            source: std::io::Error::other("disk full"),
        };
        let finish = DocumentExportError::EpubFinish {
            source: zip::result::ZipError::FileNotFound,
        };

        assert!(start.to_string().starts_with("内部错误：EPUB 写入失败: "));
        assert_eq!(write.to_string(), "内部错误：EPUB 写入失败: disk full");
        assert!(finish.to_string().starts_with("内部错误：EPUB 打包失败: "));
    }
}
