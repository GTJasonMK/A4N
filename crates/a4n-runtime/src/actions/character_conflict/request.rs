use super::{CharacterConflictChapterScope, CharacterConflictRequestContext};
use crate::{AppState, actions::access};
use a4n_core::error::AppError;

mod build;
mod preview;

pub use preview::preview_character_conflict_request;

pub(super) async fn build_character_conflict_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<CharacterConflictRequestContext, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let chapter_scope = CharacterConflictChapterScope::new(start_chapter, end_chapter)?;
    build::build_character_conflict_request_unchecked(state, project_id, name, chapter_scope).await
}
