use super::chapter_cleanup;
use crate::state::AppState;
use a4n_core::error::AppError;

pub async fn postprocess_chapter_after_content_change(
    state: &AppState,
    chapter_id: i64,
) -> Result<(), AppError> {
    let before = state
        .orchestrator
        .chapter_by_id(chapter_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;

    chapter_cleanup::cleanup_chapter_derived_data(
        state,
        before.project_id,
        before.id,
        before.chapter_number,
        false,
    )
    .await
    .map_err(|err| {
        AppError::Internal(anyhow::anyhow!(
            "章节后处理停止：清理章节派生数据失败: {err}"
        ))
    })?;

    if !before.has_available_content() {
        state
            .orchestrator
            .refresh_project_current_chapter(before.project_id)
            .await?;
        return Ok(());
    }

    state.process_chapter(chapter_id).await?;

    let chapter = state
        .orchestrator
        .chapter_by_id(chapter_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;

    state
        .orchestrator
        .refresh_project_current_chapter(chapter.project_id)
        .await?;
    state
        .orchestrator
        .sync_project_status_from_artifacts(chapter.project_id)
        .await?;
    Ok(())
}

pub async fn preflight_chapter_content_change(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), AppError> {
    chapter_cleanup::preflight_chapter_derived_cleanup(state, project_id, chapter_number)
        .await
        .map_err(|err| AppError::InvalidState {
            current: "章节派生数据无法清理".to_string(),
            operation: format!("保存章节正文前预检: {err}"),
        })
}
