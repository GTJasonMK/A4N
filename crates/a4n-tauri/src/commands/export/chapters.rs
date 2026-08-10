use a4n_runtime::host_api::export_files::{
    self, ExportFileData, ExportProjectInput, ExportPublicationInput,
};

tauri_export_read_command!(
    export_txt_data(input: ExportProjectInput) -> ExportFileData,
    export_files::export_txt_task_scope(state.inner(), user_id, input.project_id).await?,
    "正在准备 TXT 导出",
    "正在读取可导出章节",
    "TXT 导出完成",
    |state, user_id, scope| export_files::export_txt_data(state.inner(), scope),
);

tauri_export_read_command!(
    export_markdown_data(input: ExportPublicationInput) -> ExportFileData,
    export_files::export_markdown_task_scope(state.inner(), user_id, input.project_id).await?,
    "正在准备 Markdown 导出",
    "正在读取可导出章节",
    "Markdown 导出完成",
    |state, user_id, scope| export_files::export_markdown_data(state.inner(), scope, input.metadata),
);
