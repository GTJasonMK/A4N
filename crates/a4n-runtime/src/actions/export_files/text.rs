use super::{
    contracts::ExportFileData,
    rules::required_text,
    scopes::{
        export_characters_json_task_scope, export_markdown_task_scope,
        export_outline_json_task_scope, export_txt_task_scope,
    },
};
use crate::AppState;
use base64::Engine as _;

async fn build_txt_export_content(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<String, String> {
    export_txt_task_scope(state, user_id, project_id).await?;
    state
        .orchestrator
        .export_txt(project_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_txt_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    let content = build_txt_export_content(state, user_id, project_id).await?;
    Ok(export_text_file_data(
        project_id,
        "txt",
        "text/plain;charset=utf-8",
        content,
    ))
}

async fn build_markdown_export_content(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<String, String> {
    export_markdown_task_scope(state, user_id, project_id).await?;
    let title = required_text(title, "Markdown 标题")?;
    let author = required_text(author, "Markdown 作者")?;
    state
        .orchestrator
        .export_markdown(project_id, title, author)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_markdown_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<ExportFileData, String> {
    let content = build_markdown_export_content(state, user_id, project_id, title, author).await?;
    Ok(export_text_file_data(
        project_id,
        "md",
        "text/markdown;charset=utf-8",
        content,
    ))
}

async fn build_outline_json_export_value(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    export_outline_json_task_scope(state, user_id, project_id).await?;
    state
        .orchestrator
        .export_outline_json(project_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_outline_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    let value = build_outline_json_export_value(state, user_id, project_id).await?;
    let content = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    Ok(export_text_file_data(
        project_id,
        "outline.json",
        "application/json;charset=utf-8",
        content,
    ))
}

async fn build_characters_json_export_value(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    export_characters_json_task_scope(state, user_id, project_id).await?;
    state
        .orchestrator
        .export_characters_json(project_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_characters_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    let value = build_characters_json_export_value(state, user_id, project_id).await?;
    let content = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    Ok(export_text_file_data(
        project_id,
        "characters.json",
        "application/json;charset=utf-8",
        content,
    ))
}

fn export_file_name(project_id: i64, extension: &str) -> String {
    format!("{project_id}_export.{extension}")
}

fn export_text_file_data(
    project_id: i64,
    extension: &str,
    mime_type: &str,
    content: String,
) -> ExportFileData {
    export_binary_file_data(project_id, extension, mime_type, content.into_bytes())
}

pub(super) fn export_binary_file_data(
    project_id: i64,
    extension: &str,
    mime_type: &str,
    data: impl AsRef<[u8]>,
) -> ExportFileData {
    ExportFileData {
        file_name: export_file_name(project_id, extension),
        mime_type: mime_type.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(data),
    }
}
