use a4n_runtime::actions::export_files;
use a4n_runtime::actions::export_files::ExportFileData;

tauri_export_read_command!(
    export_outline_json_data(project_id: i64) -> ExportFileData,
    export_files::export_outline_json_task_scope(state.inner(), user_id, project_id).await?,
    "正在准备导出大纲 JSON",
    "正在读取大纲数据",
    "大纲 JSON 导出完成",
    "export_outline_json_failed",
    cancel: "大纲 JSON 导出已停止",
    |state, user_id, scope| export_files::export_outline_json_data(
        state.inner(),
        user_id,
        project_id,
    ),
);

tauri_export_read_command!(
    export_characters_json_data(project_id: i64) -> ExportFileData,
    export_files::export_characters_json_task_scope(state.inner(), user_id, project_id).await?,
    "正在准备导出角色 JSON",
    "正在读取角色数据",
    "角色 JSON 导出完成",
    "export_characters_json_failed",
    cancel: "角色 JSON 导出已停止",
    |state, user_id, scope| export_files::export_characters_json_data(
        state.inner(),
        user_id,
        project_id,
    ),
);
