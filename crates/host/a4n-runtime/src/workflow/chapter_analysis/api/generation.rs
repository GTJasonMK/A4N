use a4n_api_contract::chapter::ChapterSummary;

use super::super::{ChapterAnalysisProgress, execution::run_chapter_analysis_application};
use crate::state::AppState;

pub async fn preflight_generate_chapter_analysis(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ChapterSummary, String> {
    state
        .chapter_analysis_generation_application()
        .preflight_generate(user_id, chapter_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn generate_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<a4n_api_contract::chapter::ChapterAnalysisMutationResult, String> {
    run_chapter_analysis_application(state, user_id, chapter_id, progress).await
}
