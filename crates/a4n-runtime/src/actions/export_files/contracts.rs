pub use a4n_orchestrator::export::{
    BatchExportInput, ExportFileData, ExportMarkdownInput, ExportProgress, ExportProgressReporter,
};

#[derive(Debug, Clone, Copy)]
pub struct ExportTaskScope {
    pub project_id: i64,
    pub total: u32,
}
