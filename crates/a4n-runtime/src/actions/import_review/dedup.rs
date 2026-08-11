use super::{CheckImportDedupInput, DedupResult};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn check_import_dedup(
    state: &AppState,
    user_id: i64,
    input: CheckImportDedupInput,
) -> Result<DedupResult, AppError> {
    let result = state
        .orchestrator()
        .check_import_dedup_for_user_validated(user_id, input.project_id, &input.content)
        .await?;
    Ok(DedupResult {
        content_hash: result.content_hash,
        is_duplicate: result.is_duplicate,
        closest_distance: result.closest_distance,
        matched_chapter_id: result.matched_chapter_id,
    })
}
