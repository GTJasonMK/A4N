use crate::{AppState, actions::app_error_message};
use a4n_core::dependency_conflict::DownstreamConflictStrategy;

use super::{
    BindReferenceImageInput, MangaPromptSummary, RuntimeMangaDownstreamCleanup,
    UpdateMangaPromptInput, projections::to_summary,
};

pub async fn list_manga_panels(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<MangaPromptSummary>, String> {
    let panels = state
        .manga()
        .list_manga_panels_validated(user_id, project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    panels.iter().map(to_summary).collect()
}

pub async fn bind_manga_reference_image(
    state: &AppState,
    user_id: i64,
    input: BindReferenceImageInput,
) -> Result<MangaPromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    let updated = state
        .manga()
        .bind_manga_reference_image_validated(user_id, input, &cleanup)
        .await
        .map_err(app_error_message)?;
    to_summary(&updated)
}

pub async fn update_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: UpdateMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    let updated = state
        .manga()
        .update_manga_prompt_validated(user_id, input, &cleanup)
        .await
        .map_err(app_error_message)?;
    to_summary(&updated)
}

pub async fn delete_manga_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), String> {
    state
        .manga()
        .delete_manga_by_chapter_validated(user_id, project_id, chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn delete_manga_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let cleanup = RuntimeMangaDownstreamCleanup { state, user_id };
    state
        .manga()
        .delete_manga_prompt_validated(user_id, prompt_id, downstream_strategy, &cleanup)
        .await
        .map_err(app_error_message)
}
