use crate::state::AppState;
use crate::workflow::chapter_cleanup::preflight_chapter_derived_cleanup;

pub async fn validate_chapter_generation_ready(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), String> {
    state
        .orchestrator
        .validate_chapter_generation_readiness(project_id, chapter_number)
        .await?;

    preflight_chapter_derived_cleanup(state, project_id, chapter_number).await
}
