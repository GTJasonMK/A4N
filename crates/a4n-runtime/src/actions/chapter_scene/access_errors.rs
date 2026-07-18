use super::contracts::GenerateChapterSceneInput;
use crate::{AppState, actions::access};
use a4n_core::{
    error::AppError,
    models::chapter::{chapter, chapter_scene},
};

pub(super) async fn verify_scene_access(
    state: &AppState,
    user_id: i64,
    scene_id: i64,
) -> Result<chapter_scene::Model, AppError> {
    let scene = state
        .orchestrator
        .chapter_scene_by_id(scene_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("场景不存在: scene_id={scene_id}")))?;
    verify_chapter_access(state, user_id, scene.chapter_id).await?;
    Ok(scene)
}

pub(super) async fn verify_chapter_access(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, AppError> {
    let ch = state
        .orchestrator
        .chapter_by_id(chapter_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
    access::verify_project_editor(state, user_id, ch.project_id).await?;
    Ok(ch)
}

pub(super) fn required_scene_prompt_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!(
            "生成章节场景缺少{field}"
        )))
    } else {
        Ok(value)
    }
}

pub(super) fn trimmed_optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(super) fn required_updated_text(value: &str, field: &str) -> Result<String, AppError> {
    trimmed_optional_text(value)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field}不能为空")))
}

pub(super) fn required_scene_generation_inputs<'a>(
    ch: &'a chapter::Model,
    input: &'a GenerateChapterSceneInput,
) -> Result<(&'a str, &'a str, &'a str), AppError> {
    ensure_scene_number(input.scene_number)?;
    let title = required_chapter_prompt_text(
        ch.title.as_deref(),
        "标题",
        "生成章节场景",
        ch.chapter_number,
    )?;
    let summary = required_chapter_prompt_text(
        ch.summary.as_deref(),
        "摘要",
        "生成章节场景",
        ch.chapter_number,
    )?;
    let scene_prompt = required_scene_prompt_text(&input.scene_prompt, "场景引导")?;
    Ok((title, summary, scene_prompt))
}

fn required_chapter_prompt_text<'a>(
    value: Option<&'a str>,
    field: &str,
    action: &str,
    chapter_number: i32,
) -> Result<&'a str, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{action}需要第 {chapter_number} 章已有{field}"))
        })
}

pub(super) fn ensure_scene_number(scene_number: i32) -> Result<(), AppError> {
    if scene_number <= 0 {
        return Err(AppError::InvalidArgument("场景编号必须大于 0".to_string()));
    }
    Ok(())
}
