use super::RuntimeMangaDownstreamCleanup;
use crate::{AppState, actions::app_error_message};
use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPromptInput, MangaChapterInput,
    MangaPagePromptSummary, UpdateMangaPromptInput,
};

pub async fn list_manga_page_prompts(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<Vec<MangaPagePromptSummary>, String> {
    state
        .manga_record_read_application()
        .list_page_prompts(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn bind_manga_reference_image(
    state: &AppState,
    user_id: i64,
    input: BindMangaReferenceImageInput,
) -> Result<MangaPagePromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    state
        .manga_record_mutation_application()
        .bind_reference_image(user_id, input, &cleanup)
        .await
        .map_err(app_error_message)
}

pub async fn update_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: UpdateMangaPromptInput,
) -> Result<MangaPagePromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    state
        .manga_record_mutation_application()
        .update_prompt(user_id, input, &cleanup)
        .await
        .map_err(app_error_message)
}

pub async fn delete_manga_by_chapter(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<(), String> {
    state
        .manga_record_mutation_application()
        .delete_by_chapter(user_id, input.project_id, input.chapter_id)
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
        .manga_record_mutation_application()
        .delete_prompt(user_id, input, &cleanup)
        .await
        .map_err(app_error_message)
}
