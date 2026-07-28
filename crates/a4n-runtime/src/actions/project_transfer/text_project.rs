//! 文本项目完整导入 Host 适配：文件名/Base64/UTF-8；校验与落库归 orch。

use super::{
    contracts::ImportProjectFullUploadInput,
    package_files::{decode_uploaded_project_file, validate_uploaded_project_file_name},
};
use crate::state::AppState;
use a4n_core::models::project::novel_project;
use serde_json::Value;

pub async fn import_project_full_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectFullUploadInput,
) -> Result<novel_project::Model, String> {
    validate_uploaded_project_file_name(&input.file_name, "json", "文本项目备份")?;
    let bytes = decode_uploaded_project_file(&input.data_base64, "文本项目备份")?;
    let content =
        String::from_utf8(bytes).map_err(|err| format!("文本项目备份必须是 UTF-8 JSON：{err}"))?;
    import_project_full_from_json_str(state, user_id, &content).await
}

async fn import_project_full_from_json_str(
    state: &AppState,
    user_id: i64,
    content: &str,
) -> Result<novel_project::Model, String> {
    let root: Value =
        serde_json::from_str(content).map_err(|e| format!("解析 JSON 失败: {}", e))?;
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .project_mutation()
        .run_import_text_project_validated(&host_port, user_id, root)
        .await
        .map_err(|error| error.to_string())
}
