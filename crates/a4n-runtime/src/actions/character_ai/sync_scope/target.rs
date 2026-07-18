use crate::state::AppState;
use a4n_core::models::chapter::chapter;

pub(crate) async fn resolve_character_sync_target_chapter(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<i32, String> {
    let chapters = state
        .orchestrator
        .character_sync_chapters(project_id)
        .await
        .map_err(|err| err.to_string())?;
    validate_character_sync_target_chapter(&chapters, project_id, chapter_number)
}

pub(in crate::actions::character_ai) fn validate_character_sync_target_chapter(
    chapters: &[chapter::Model],
    project_id: i64,
    chapter_number: i32,
) -> Result<i32, String> {
    if chapter_number <= 0 {
        return Err("同步目标章节必须大于 0".to_string());
    }
    let latest_chapter = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .ok_or_else(|| format!("项目还没有可同步章节: project_id={project_id}"))?;
    if chapter_number > latest_chapter {
        return Err(format!(
            "同步目标章节不存在: 第 {chapter_number} 章，当前最新章节为第 {latest_chapter} 章"
        ));
    }
    if chapters
        .iter()
        .any(|chapter| chapter.chapter_number == chapter_number)
    {
        Ok(chapter_number)
    } else {
        Err(format!(
            "第 {chapter_number} 章没有可同步正文，请先生成或导入正文"
        ))
    }
}
