use super::super::{
    MangaPagePromptSummary, OptimizeMangaPromptInput, RuntimeMangaDownstreamCleanup,
    projections::to_summary,
};
use crate::AppState;

pub async fn optimize_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: OptimizeMangaPromptInput,
) -> Result<MangaPagePromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    let prompt_renderer = state;
    let updated = state
        .orchestrator()
        .optimize_manga_prompt_record_validated(user_id, input, prompt_renderer, &cleanup)
        .await
        .map_err(crate::actions::app_error_message)?;
    to_summary(&updated)
}
