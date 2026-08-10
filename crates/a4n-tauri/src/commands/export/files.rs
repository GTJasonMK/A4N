use a4n_runtime::host_api::export_files::{
    self, BatchExportInput, ExportFileData, ExportProjectInput, ExportPublicationInput,
    export_batch_task_scope, export_epub_task_scope, export_pdf_task_scope,
};

tauri_export_progress_command!(
    export_epub_data(input: ExportPublicationInput) -> ExportFileData,
    export_epub_task_scope(state.inner(), user_id, input.project_id).await?,
    "正在准备 EPUB 导出",
    |state, user_id, scope, progress| export_files::export_epub_data(
        state.inner(),
        scope,
        input.metadata,
        progress,
    ),
);

tauri_export_progress_command!(
    export_pdf_data(input: ExportProjectInput) -> ExportFileData,
    export_pdf_task_scope(state.inner(), user_id, input.project_id).await?,
    "正在准备 PDF 导出",
    |state, user_id, scope, progress| export_files::export_pdf_data(
        state.inner(),
        scope,
        progress,
    ),
);

tauri_export_progress_command!(
    export_batch_data(input: BatchExportInput) -> Vec<ExportFileData>,
    export_batch_task_scope(state.inner(), user_id, input.project_id).await?,
    "正在准备批量导出",
    |state, user_id, scope, progress| export_files::export_batch_data(
        state.inner(),
        scope,
        input.formats,
        input.metadata,
        progress,
    ),
);
