use crate::{actions::app_error_message, state::AppState};
use a4n_core::models::{chapter::chapter, project::text_anchor};

pub(super) async fn select_object_discovery_chapters(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if project_id <= 0 {
        return Err("物体档案发现 project_id 必须大于 0".to_string());
    }
    if chapter_number.is_some_and(|value| value <= 0) {
        return Err("物体档案发现章节号必须大于 0".to_string());
    }
    let mut chapters = state
        .orchestrator
        .character_sync_chapters(project_id)
        .await
        .map_err(app_error_message)?;
    if let Some(chapter_number) = chapter_number {
        chapters.retain(|chapter| chapter.chapter_number == chapter_number);
        if chapters.is_empty() {
            return Err(format!(
                "第 {chapter_number} 章没有可用于物体档案发现的有效正文"
            ));
        }
        return Ok(chapters);
    }
    if chapters.is_empty() {
        return Err(format!(
            "项目还没有可用于物体档案发现的章节: project_id={project_id}"
        ));
    }
    Ok(chapters)
}

pub(super) async fn read_current_chapter_content_anchors(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<text_anchor::Model>, String> {
    let rows = state
        .orchestrator
        .read_current_chapter_content_anchor_records(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    if rows.anchors.is_empty() {
        return Err(format!(
            "状态无效：当前状态 第 {} 章缺少当前正文文本锚点，不允许操作 预览模型请求",
            rows.chapter.chapter_number
        ));
    }
    if rows
        .anchors
        .iter()
        .any(|anchor| anchor.source_version_hash != rows.source_version_hash)
    {
        return Err(format!(
            "状态无效：当前状态 第 {} 章正文文本锚点已过期，不允许操作 预览模型请求",
            rows.chapter.chapter_number
        ));
    }
    Ok(rows.anchors)
}
