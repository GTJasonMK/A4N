use crate::{AppState, actions::app_error_message};
use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPromptInput, MangaChapterInput,
    MangaPagePromptSummary, UpdateMangaPromptInput,
};
use a4n_orchestrator::manga::manga_page_prompt_summary;

use super::RuntimeMangaDownstreamCleanup;

pub async fn list_manga_page_prompts(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<Vec<MangaPagePromptSummary>, String> {
    let page_prompts = state
        .orchestrator()
        .list_manga_page_prompts_validated(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)?;
    page_prompts.iter().map(manga_page_prompt_summary).collect()
}

pub async fn bind_manga_reference_image(
    state: &AppState,
    user_id: i64,
    input: BindMangaReferenceImageInput,
) -> Result<MangaPagePromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    let updated = state
        .orchestrator()
        .bind_manga_reference_image_validated(user_id, input, &cleanup)
        .await
        .map_err(app_error_message)?;
    manga_page_prompt_summary(&updated)
}

pub async fn update_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: UpdateMangaPromptInput,
) -> Result<MangaPagePromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    let updated = state
        .orchestrator()
        .update_manga_prompt_validated(user_id, input, &cleanup)
        .await
        .map_err(app_error_message)?;
    manga_page_prompt_summary(&updated)
}

pub async fn delete_manga_by_chapter(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<(), String> {
    state
        .orchestrator()
        .delete_manga_by_chapter_validated(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn delete_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: DeleteMangaPromptInput,
) -> Result<(), String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    state
        .orchestrator()
        .delete_manga_prompt_validated(
            user_id,
            input.prompt_id,
            input.downstream_strategy,
            &cleanup,
        )
        .await
        .map_err(app_error_message)
}
