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
    #[error("导出合同无效：{0}")]
    Contract(#[from] a4n_export_contract::ExportContractError),
    #[error("导出归档失败：{0}")]
    Archive(#[from] a4n_archive_core::ArchiveWriteError),
    #[error("文档格式渲染失败：{0}")]
    Document(#[from] a4n_document_export_core::DocumentExportError),
    #[error("文档导出输入无效：{0}")]
    InvalidDocument(String),
}
