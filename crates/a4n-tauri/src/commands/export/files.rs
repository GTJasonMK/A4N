use a4n_runtime::host_api::export_files::{
    self, BatchExportInput, ExportFileData, export_batch_task_scope, export_epub_task_scope,
    export_pdf_task_scope,
};

tauri_export_progress_command!(
    export_epub_data(project_id: i64, title: String, author: String) -> ExportFileData,
    export_epub_task_scope(state.inner(), user_id, project_id).await?,
    "正在准备 EPUB 导出",
    "EPUB 导出完成",
    "export_epub_failed",
    |state, user_id, scope, progress| export_files::export_epub_data(
        state.inner(),
        user_id,
        project_id,
        title,
        author,
        progress,
    ),
);

tauri_export_progress_command!(
    export_pdf_data(project_id: i64) -> ExportFileData,
    export_pdf_task_scope(state.inner(), user_id, project_id).await?,
    "正在准备 PDF 导出",
    "PDF 导出完成",
    "export_pdf_failed",
    |state, user_id, scope, progress| export_files::export_pdf_data(
        state.inner(),
        user_id,
        project_id,
        progress,
    ),
);

tauri_export_progress_command!(
    export_batch_data(input: BatchExportInput) -> Vec<ExportFileData>,
    export_batch_task_scope(state.inner(), user_id, &input).await?,
    "正在准备批量导出",
    "批量导出完成",
    "export_batch_failed",
    |state, user_id, scope, progress| export_files::export_batch_data(
        state.inner(),
        user_id,
        input,
        progress,
    ),
);
