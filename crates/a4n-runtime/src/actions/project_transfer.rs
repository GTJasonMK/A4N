mod family_package;
mod import_payload;
mod import_writer;
mod validation;
mod video_studio_package;

use self::family_package::{
    export_derived_project_package_bytes, export_project_family_package_bytes,
    import_project_family_package_bytes,
};
use self::import_payload::parse_import_payload;
use self::import_writer::{
    insert_import_blueprint, insert_import_blueprint_versions, insert_import_chapter_outlines,
    insert_import_characters, insert_import_part_outlines, insert_import_relationships,
};
use self::validation::{
    optional_json_positive_i32_field, optional_json_string, required_json_nonnegative_i32_field,
    required_json_object, required_json_positive_i32_field, required_json_text,
};
use self::video_studio_package::{
    export_video_studio_project_package_bytes, import_video_studio_project_package_bytes,
};
use crate::actions::access;
use crate::state::AppState;
use a4n_core::models::{chapter::chapter, project::novel_project};
use a4n_core::state::validator::StateValidator;
use a4n_services::project::normalize_project_type;
use base64::Engine as _;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectFullUploadInput {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportProjectFamilyDataInput {
    pub root_project_id: i64,
    pub include_derived_projects: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportDerivedProjectDataInput {
    pub derived_project_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportVideoStudioProjectDataInput {
    pub project_id: i64,
}

#[derive(Debug, Serialize)]
pub struct ExportProjectPackageData {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectFamilyUploadInput {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportVideoStudioProjectUploadInput {
    pub file_name: String,
    pub data_base64: String,
}

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
    let project_type = normalize_project_type(project_type).map_err(|e| e.to_string())?;
    let status = required_json_text(project_data, "status", "项目状态")?;
    StateValidator::parse_status(status).map_err(|e| format!("导入项目状态无效: {e}"))?;
    let target_chapters =
        required_json_positive_i32_field(project_data, "target_chapters", "目标章节数")?;
    let current_chapter =
        required_json_nonnegative_i32_field(project_data, "current_chapter", "当前章节数")?;
    let total_chapters =
        optional_json_positive_i32_field(project_data, "total_chapters", "总章节数")?;
    let chapters_per_part =
        optional_json_positive_i32_field(project_data, "chapters_per_part", "分部章节数")?;
    let imported_payload = parse_import_payload(&json)?;
    let (max_chapter_number, max_available_content_chapter_number, available_content_chapter_count) =
        imported_payload.chapter_bound_counts()?;

    validate_import_project_chapter_bounds(
        target_chapters,
        current_chapter,
        total_chapters,
        chapters_per_part,
        max_chapter_number,
        max_available_content_chapter_number,
        available_content_chapter_count,
    )?;

    let txn = state.db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now().naive_utc();
    let project = novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(title.to_string()),
        description: Set(description.to_string()),
        project_type: Set(project_type.to_string()),
        project_kind: Set(a4n_services::project::PROJECT_KIND_TEXT.to_string()),
        source_mode: Set(a4n_services::project::SOURCE_MODE_SELF.to_string()),
        root_project_id: Set(None),
        parent_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(a4n_services::project::SOURCE_SYNC_CURRENT.to_string()),
        derived_project_status: Set(None),
        status: Set(status.to_string()),
        target_chapters: Set(target_chapters),
        current_chapter: Set(current_chapter),
        total_chapters: Set(total_chapters),
        chapters_per_part: Set(chapters_per_part),
        avatar_svg: Set(
            optional_json_string(project_data, "avatar_svg", "项目头像 SVG")?
                .map(ToString::to_string),
        ),
        avatar_animal: Set(
            optional_json_string(project_data, "avatar_animal", "项目头像")?
                .map(ToString::to_string),
        ),
        raw_blueprint: Set(
            optional_json_string(project_data, "raw_blueprint", "项目蓝图")?
                .map(ToString::to_string),
        ),
        import_analysis_progress: Set(optional_json_string(
            project_data,
            "import_analysis_progress",
            "导入分析进度",
        )?
        .map(ToString::to_string)),
        export_config: Set(
            optional_json_string(project_data, "export_config", "导出配置")?
                .map(ToString::to_string),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| format!("导入项目失败: {e}"))?;

    let part_id_map =
        insert_import_part_outlines(&txn, project.id, &imported_payload.part_outlines, now).await?;
    let outline_id_map = insert_import_chapter_outlines(
        &txn,
        project.id,
        &imported_payload.chapter_outlines,
        &part_id_map,
        now,
    )
    .await?;
    if let Some(blueprint) = &imported_payload.blueprint {
        insert_import_blueprint(&txn, project.id, blueprint, now).await?;
    }
    insert_import_blueprint_versions(&txn, project.id, &imported_payload.blueprint_versions, now)
        .await?;
    insert_import_characters(&txn, project.id, &imported_payload.characters, now).await?;
    insert_import_relationships(&txn, project.id, &imported_payload.relationships, now).await?;

    let mut postprocess_chapter_ids = Vec::new();

    for import_chapter in &imported_payload.chapters {
        let chapter_label = format!("第{}章", import_chapter.chapter_number);
        let outline_id = import_chapter
            .old_outline_id
            .map(|old_id| {
                outline_id_map.get(&old_id).copied().ok_or_else(|| {
                    format!("{chapter_label}引用的章节大纲不存在: outline_id={old_id}")
                })
            })
            .transpose()?;
        let chapter = chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(outline_id),
            chapter_number: Set(import_chapter.chapter_number),
            title: Set(import_chapter.title.clone()),
            content: Set(import_chapter.content.clone()),
            word_count: Set(import_chapter.word_count),
            status: Set(import_chapter.status.clone()),
            summary: Set(import_chapter.summary.clone()),
            style_tags: Set(import_chapter.style_tags.clone()),
            analysis_data: Set(import_chapter.analysis_data.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|e| format!("导入章节失败: {}", e))?;
        postprocess_chapter_ids.push(chapter.id);
    }

    txn.commit().await.map_err(|e| e.to_string())?;

    if !imported_payload.has_preserved_chapter_tags {
        for chapter_id in postprocess_chapter_ids {
            crate::workflow::postprocess_chapter_after_content_change(state, chapter_id)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    let project = state
        .project
        .get(project.id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(project)
}

pub async fn export_project_family_data(
    state: &AppState,
    user_id: i64,
    input: ExportProjectFamilyDataInput,
) -> Result<ExportProjectPackageData, String> {
    access::verify_project_owner_string(state, user_id, input.root_project_id).await?;
    let project = state
        .project
        .get(input.root_project_id)
        .await
        .map_err(|e| e.to_string())?;
    let bytes = export_project_family_package_bytes(
        state,
        input.root_project_id,
        input.include_derived_projects,
    )
    .await?;
    Ok(export_package_data(&project, bytes))
}

pub async fn export_derived_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportDerivedProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    access::verify_project_owner_string(state, user_id, input.derived_project_id).await?;
    let project = state
        .project
        .get(input.derived_project_id)
        .await
        .map_err(|e| e.to_string())?;
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("派生项目缺少 root_project_id: {}", project.id))?;
    access::verify_project_owner_string(state, user_id, root_project_id).await?;
    let bytes = export_derived_project_package_bytes(state, input.derived_project_id).await?;
    Ok(export_package_data(&project, bytes))
}

pub async fn import_project_family_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectFamilyUploadInput,
) -> Result<Vec<novel_project::Model>, String> {
    validate_uploaded_project_file_name(&input.file_name, "zip", "项目族包")?;
    let bytes = decode_uploaded_project_file(&input.data_base64, "项目族包")?;
    import_project_family_package_bytes(state, user_id, bytes).await
}

pub async fn export_video_studio_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportVideoStudioProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    access::verify_project_owner_string(state, user_id, input.project_id).await?;
    let project = state
        .project
        .get(input.project_id)
        .await
        .map_err(|e| e.to_string())?;
    let bytes = export_video_studio_project_package_bytes(state, input.project_id).await?;
    Ok(export_package_data(&project, bytes))
}

pub async fn import_video_studio_project_upload(
    state: &AppState,
    user_id: i64,
    input: ImportVideoStudioProjectUploadInput,
) -> Result<novel_project::Model, String> {
    validate_uploaded_project_file_name(&input.file_name, "zip", "视频画布项目包")?;
    let bytes = decode_uploaded_project_file(&input.data_base64, "视频画布项目包")?;
    import_video_studio_project_package_bytes(state, user_id, bytes).await
}

fn export_package_data(project: &novel_project::Model, bytes: Vec<u8>) -> ExportProjectPackageData {
    ExportProjectPackageData {
        file_name: project_package_file_name(project),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    }
}

fn project_package_file_name(project: &novel_project::Model) -> String {
    let stem = sanitize_file_name_component(&project.title);
    let stem = if stem.is_empty() {
        format!("project-{}", project.id)
    } else {
        stem
    };
    format!("{stem}.a4n-project.zip")
}

fn sanitize_file_name_component(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string()
}

fn validate_uploaded_project_file_name(
    file_name: &str,
    expected_extension: &str,
    label: &str,
) -> Result<(), String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}文件名不能为空"));
    }
    let name = std::path::Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label}文件名无效"))?;
    if name != trimmed {
        return Err(format!("{label}文件名不能包含路径"));
    }
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .ok_or_else(|| format!("{label}必须是 .{expected_extension} 文件"))?;
    if !extension.eq_ignore_ascii_case(expected_extension) {
        return Err(format!("{label}必须是 .{expected_extension} 文件"));
    }
    Ok(())
}

fn decode_uploaded_project_file(data_base64: &str, label: &str) -> Result<Vec<u8>, String> {
    let raw = data_base64
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(data_base64)
        .trim();
    if raw.is_empty() {
        return Err(format!("{label}内容不能为空"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("{label} base64 解码失败：{err}"))?;
    if bytes.is_empty() {
        return Err(format!("{label}内容不能为空"));
    }
    Ok(bytes)
}

fn validate_import_project_chapter_bounds(
    target_chapters: i32,
    current_chapter: i32,
    total_chapters: Option<i32>,
    chapters_per_part: Option<i32>,
    max_chapter_number: i32,
    max_available_content_chapter_number: i32,
    available_content_chapter_count: i32,
) -> Result<(), String> {
    let effective_total = total_chapters.unwrap_or(target_chapters);
    if current_chapter > effective_total {
        return Err(format!(
            "当前章节数不能大于总章节数: current_chapter={current_chapter}, total={effective_total}"
        ));
    }
    if max_chapter_number > effective_total {
        return Err(format!(
            "导入章节序号不能大于总章节数: max_chapter_number={max_chapter_number}, total={effective_total}"
        ));
    }
    if current_chapter != max_available_content_chapter_number {
        return Err(format!(
            "当前章节数必须等于导入可用正文中的最高章节序号: current_chapter={current_chapter}, max_content_chapter={max_available_content_chapter_number}"
        ));
    }
    if current_chapter != available_content_chapter_count {
        return Err(format!(
            "导入可用正文必须从第1章连续到当前章节: current_chapter={current_chapter}, available_content_chapters={available_content_chapter_count}"
        ));
    }
    if let Some(chapters_per_part) = chapters_per_part
        && chapters_per_part > effective_total
    {
        return Err(format!(
            "分部章节数不能大于总章节数: chapters_per_part={chapters_per_part}, total={effective_total}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_import_project_chapter_bounds_rejects_content_gap() {
        let err = validate_import_project_chapter_bounds(5, 3, None, None, 3, 3, 2)
            .expect_err("available imported content must be continuous from chapter 1");

        assert!(err.contains("导入可用正文必须从第1章连续到当前章节"));
        assert!(err.contains("current_chapter=3"));
        assert!(err.contains("available_content_chapters=2"));
    }

    #[test]
    fn validate_import_project_chapter_bounds_accepts_contiguous_content() {
        validate_import_project_chapter_bounds(5, 3, None, None, 3, 3, 3)
            .expect("chapters 1..=current_chapter should be a valid import boundary");
    }

    #[test]
    fn validate_import_project_chapter_bounds_accepts_deleted_content_beyond_current() {
        validate_import_project_chapter_bounds(5, 1, None, None, 2, 1, 1).expect(
            "deleted chapter records may exist beyond current_chapter without becoming available content",
        );
    }
}
