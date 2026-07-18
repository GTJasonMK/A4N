use super::{
    contracts::ImportProjectFullUploadInput,
    import_payload::parse_import_payload,
    package_files::{decode_uploaded_project_file, validate_uploaded_project_file_name},
    validation::{
        optional_json_positive_i32_field, optional_json_string,
        required_json_nonnegative_i32_field, required_json_object,
        required_json_positive_i32_field, required_json_text,
    },
};
use crate::state::AppState;
use a4n_core::models::project::novel_project;
use a4n_orchestrator::project::ImportTextProjectRecordInput;
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
    let json: Value =
        serde_json::from_str(content).map_err(|e| format!("解析 JSON 失败: {}", e))?;

    let project_data = required_json_object(&json, "project", "导入文件")?;
    let title = required_json_text(project_data, "title", "项目标题")?;
    let description = required_json_text(project_data, "description", "项目简介")?;
    let project_type = required_json_text(project_data, "project_type", "项目类型")?;
    let status = required_json_text(project_data, "status", "项目状态")?;
    let target_chapters =
        required_json_positive_i32_field(project_data, "target_chapters", "目标章节数")?;
    let current_chapter =
        required_json_nonnegative_i32_field(project_data, "current_chapter", "当前章节数")?;
    let total_chapters =
        optional_json_positive_i32_field(project_data, "total_chapters", "总章节数")?;
    let chapters_per_part =
        optional_json_positive_i32_field(project_data, "chapters_per_part", "分部章节数")?;
    let imported_payload = parse_import_payload(&json)?;
    let import_result = state
        .orchestrator
        .import_text_project_record(ImportTextProjectRecordInput {
            user_id,
            title: title.to_string(),
            description: description.to_string(),
            project_type: project_type.to_string(),
            status: status.to_string(),
            target_chapters,
            current_chapter,
            total_chapters,
            chapters_per_part,
            avatar_svg: optional_json_string(project_data, "avatar_svg", "项目头像 SVG")?
                .map(ToString::to_string),
            avatar_animal: optional_json_string(project_data, "avatar_animal", "项目头像")?
                .map(ToString::to_string),
            raw_blueprint: optional_json_string(project_data, "raw_blueprint", "项目蓝图")?
                .map(ToString::to_string),
            import_analysis_progress: optional_json_string(
                project_data,
                "import_analysis_progress",
                "导入分析进度",
            )?
            .map(ToString::to_string),
            export_config: optional_json_string(project_data, "export_config", "导出配置")?
                .map(ToString::to_string),
            payload: imported_payload.into_db_payload(),
        })
        .await
        .map_err(|error| error.to_string())?;

    if !import_result.has_preserved_chapter_tags {
        for chapter_id in import_result.postprocess_chapter_ids.iter().copied() {
            crate::workflow::postprocess_chapter_after_content_change(state, chapter_id)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(import_result.project)
}
