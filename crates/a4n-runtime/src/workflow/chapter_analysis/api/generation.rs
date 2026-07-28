use a4n_core::models::chapter::chapter;

use super::super::{ChapterAnalysisProgress, execution::run_chapter_analysis_application};
use crate::{actions::access, state::AppState};

pub async fn preflight_generate_chapter_analysis(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    let chapter = state
        .chapter_analysis()
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
) -> Result<chapter::Model, String> {
    preflight_generate_chapter_analysis(state, user_id, chapter_id).await?;
    run_chapter_analysis_application(state, chapter_id, progress, true).await
}
