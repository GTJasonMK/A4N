use super::super::RuntimeMangaDownstreamCleanup;
use crate::AppState;
use a4n_api_contract::manga::{MangaPagePromptSummary, OptimizeMangaPromptInput};

pub async fn optimize_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: OptimizeMangaPromptInput,
) -> Result<MangaPagePromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    let prompt_renderer = state;
    state
        .manga_record_application()
        .optimize_prompt(user_id, input, prompt_renderer, &cleanup)
        .await
        .map_err(crate::actions::app_error_message)
}
