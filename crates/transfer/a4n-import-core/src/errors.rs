#[derive(Debug, thiserror::Error)]
pub enum ImportTransformError {
    #[error("{0}")]
    Contract(#[from] a4n_import_contract::ImportContractError),
    #[error("{0}")]
    InvalidDocument(String),
    #[error("导入 JSON 解析失败：{0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("读取{label} zip 失败：{source}")]
    OpenZip {
        label: String,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("{label}解析失败：{source}")]
    InvalidManifestJson {
        label: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{label}格式无效：{message}")]
    InvalidManifestShape { label: String, message: String },
    #[error("{0}")]
    InvalidZipEntryPath(#[from] a4n_archive_core::ArchivePathError),
    #[error("{label} base64 解码失败：{source}")]
    Base64Decode {
        label: String,
        #[source]
        source: base64::DecodeError,
    },
    #[error("{label}缺少 zip 条目 {path}：{source}")]
    MissingZipEntry {
        label: String,
        path: String,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("读取{label} zip 条目失败：{path}：{source}")]
    ReadZipEntry {
        label: String,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("读取{label} zip 条目失败：index={index}：{source}")]
    ReadZipEntryByIndex {
        label: String,
        index: usize,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("{label} zip 条目重复：{path}")]
    DuplicateZipEntry { label: String, path: String },
    #[error("{label}清单重复声明 zip 文件：{path}")]
    DuplicateDeclaredZipEntry { label: String, path: String },
    #[error("{label}包含未在清单声明的文件：{path}")]
    UndeclaredZipEntry { label: String, path: String },
}
