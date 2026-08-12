use a4n_api_contract::chapter::ChapterAnalysisMutationResult;
use a4n_core::models::chapter::chapter;

use super::super::{ChapterAnalysisProgress, execution::run_chapter_analysis_application};
use crate::{
    actions::{access, chapter_content::chapter_summary_with_sync_state},
    state::AppState,
};

pub async fn preflight_generate_chapter_analysis(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    let chapter = state
        .orchestrator()
        .preflight_chapter_analysis_validated(chapter_id, false)
        .await
        .map_err(|error| error.to_string())?;
    access::verify_project_editor(state, user_id, chapter.project_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(chapter)
}

pub async fn generate_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<ChapterAnalysisMutationResult, String> {
    preflight_generate_chapter_analysis(state, user_id, chapter_id).await?;
    match run_chapter_analysis_application(state, chapter_id, progress).await {
        Ok(chapter) => Ok(ChapterAnalysisMutationResult {
            chapter: chapter_summary_with_sync_state(state, &chapter)
                .await
                .map_err(|error| error.to_string())?,
            warnings: Vec::new(),
        }),
        Err(sync_error) => {
            let chapter = state
                .orchestrator()
                .chapter_by_id(chapter_id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("章节不存在: chapter_id={chapter_id}"))?;
            let summary = chapter_summary_with_sync_state(state, &chapter)
                .await
                .map_err(|error| error.to_string())?;
            if !summary.analysis_sync_has_saved_analysis || !summary.analysis_sync_can_resume {
                return Err(sync_error);
            }
            Ok(ChapterAnalysisMutationResult {
                chapter: summary,
                warnings: vec![format!(
                    "结构化分析已保存，但后续资料同步未完成：{sync_error}"
                )],
            })
        }
    }
}
