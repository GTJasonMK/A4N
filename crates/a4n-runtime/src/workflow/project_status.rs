use crate::state::AppState;

pub async fn validate_chapter_generation_ready(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), String> {
    state
        .project_chapter_records()
        .validate_chapter_generation_readiness(project_id, chapter_number)
        .await?;

    state
        .chapter_content()
        .preflight_chapter_content_change_validated(project_id, chapter_number)
        .await
        .map_err(|error| error.to_string())
}
