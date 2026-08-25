use super::super::RuntimeMangaDownstreamCleanup;
use crate::AppState;
use a4n_api_contract::manga::{MangaPagePromptSummary, OptimizeMangaPromptInput};
use a4n_orchestrator::manga::manga_page_prompt_summary;

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
    manga_page_prompt_summary(&updated)
}
