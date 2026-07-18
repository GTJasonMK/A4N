use super::super::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisProgress,
    generate_and_sync_chapter_analysis_with_progress, preflight_chapter_analysis,
    report_chapter_analysis_progress,
};
use crate::state::AppState;
use crate::workflow::postprocess_chapter_after_content_change;
use a4n_core::models::chapter::chapter;

pub async fn preflight_generate_chapter_analysis(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    preflight_chapter_analysis(
        state,
        user_id,
        chapter_id,
        "生成结构化分析",
        "章节正文为空，不能生成结构化分析",
        false,
    )
    .await
}

pub async fn generate_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    report_chapter_analysis_progress(
        progress,
        0,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在检查章节正文",
        "章节结构化分析已停止",
    )?;
    let ch = preflight_generate_chapter_analysis(state, user_id, chapter_id).await?;

    report_chapter_analysis_progress(
        progress,
        5,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在清理旧的章节派生数据",
        "章节结构化分析已停止",
    )?;
    postprocess_chapter_after_content_change(state, ch.id)
        .await
        .map_err(|err| err.to_string())?;

    generate_and_sync_chapter_analysis_with_progress(state, user_id, ch.id, progress).await
}
