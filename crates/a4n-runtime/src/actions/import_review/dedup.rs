use super::{CheckImportDedupInput, DedupResult};
use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use a4n_import_core::dedup;

pub async fn check_import_dedup(
    state: &AppState,
    user_id: i64,
    input: CheckImportDedupInput,
) -> Result<DedupResult, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let content_hash = dedup::compute_simhash(&input.content, 3);
    let existing_chapters = state
        .orchestrator
        .import_dedup_chapters(input.project_id)
        .await?;

    let threshold = 3;
    let mut closest_distance = u32::MAX;
    let mut matched_chapter_id = None;
    for chapter in existing_chapters {
        let existing_hash = dedup::compute_simhash(&chapter.content, 3);
        let distance = content_hash.hamming_distance(&existing_hash);
        if distance < closest_distance {
            closest_distance = distance;
        }
        if distance <= threshold && matched_chapter_id.is_none() {
            matched_chapter_id = Some(chapter.id);
        }
    }

    Ok(DedupResult {
        content_hash: format!("{:016x}", content_hash.0),
        is_duplicate: matched_chapter_id.is_some(),
        closest_distance,
        matched_chapter_id,
    })
}
