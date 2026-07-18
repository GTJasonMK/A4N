use super::metadata::image_source_metadata;
use crate::AppState;
use crate::actions::app_error_message;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageChapterBindingAccess {
    ReadOnly,
    Writable,
}

pub async fn resolve_image_chapter_binding(
    state: &AppState,
    project_id: i64,
    chapter_id: Option<i64>,
    access: ImageChapterBindingAccess,
) -> Result<(Option<i64>, Option<Value>), String> {
    if project_id <= 0 {
        return Err("project_id 必须大于 0".to_string());
    }
    let project = state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;
    let Some(id) = chapter_id else {
        return Ok((None, None));
    };
    let chapter = state
        .orchestrator
        .chapter_by_id(id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("章节 #{id} 不存在"))?;
    let source_content = match project.project_kind.as_str() {
        "text" => {
            if chapter.project_id != project.id {
                return Err(format!("章节 #{} 不属于文本项目 {project_id}", chapter.id));
            }
            if chapter.status == "deleted" {
                return Err(format!(
                    "第 {} 章已删除，不能绑定图片素材",
                    chapter.chapter_number
                ));
            }
            if access == ImageChapterBindingAccess::Writable && chapter.status == "generating" {
                return Err(format!(
                    "第 {} 章正在生成中，请等待完成后再绑定图片素材",
                    chapter.chapter_number
                ));
            }
            chapter.content.clone()
        }
        "comic" | "video" | "galgame" => {
            if project.root_project_id != Some(chapter.project_id) {
                return Err(format!(
                    "章节 #{} 不属于派生项目 {project_id} 绑定的根文本项目",
                    chapter.id
                ));
            }
            let snapshot_id = project
                .source_snapshot_id
                .ok_or_else(|| format!("派生项目缺少源文本快照: {project_id}"))?;
            state
                .orchestrator
                .source_snapshot_chapter_by_snapshot_and_source(snapshot_id, chapter.id)
                .await
                .map_err(app_error_message)?
                .ok_or_else(|| format!("派生项目 {project_id} 的源快照不包含章节 {}", chapter.id))?
                .content
        }
        other => {
            return Err(format!(
                "图片链路不支持项目类型 {other}: project_id={project_id}"
            ));
        }
    };
    let source_metadata = if source_content.trim().is_empty() {
        None
    } else {
        Some(image_source_metadata(&source_content))
    };
    Ok((Some(chapter.id), source_metadata))
}
