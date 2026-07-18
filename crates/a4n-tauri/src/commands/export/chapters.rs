use a4n_runtime::host_api::export_files::{self, ExportFileData, ExportMarkdownInput};

tauri_export_read_command!(
    export_txt_data(project_id: i64) -> ExportFileData,
    export_files::export_txt_task_scope(state.inner(), user_id, project_id).await?,
    "正在准备 TXT 导出",
    "正在读取可导出章节",
    "TXT 导出完成",
    "export_txt_failed",
    |state, user_id, scope| export_files::export_txt_data(state.inner(), user_id, project_id),
);

tauri_export_read_command!(
    export_markdown_data(project_id: i64, input: ExportMarkdownInput) -> ExportFileData,
    export_files::export_markdown_task_scope(state.inner(), user_id, project_id).await?,
    "正在准备 Markdown 导出",
    "正在读取可导出章节",
    "Markdown 导出完成",
    "export_markdown_failed",
    |state, user_id, scope| export_files::export_markdown_data(
        state.inner(),
        user_id,
        project_id,
        &input.title,
        &input.author,
    ),
);
