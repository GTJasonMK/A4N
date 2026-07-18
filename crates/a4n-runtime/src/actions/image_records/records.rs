use crate::{
    AppState,
    actions::{
        access, app_error_message,
        image_generation::{GeneratedImageSummary, to_summary},
    },
};
use a4n_orchestrator::video::UpdateGeneratedImageRecordInput as UpdateImageRecord;

use super::UpdateGeneratedImageInput;

pub async fn update_image(
    state: &AppState,
    user_id: i64,
    image_id: i64,
    input: UpdateGeneratedImageInput,
) -> Result<GeneratedImageSummary, String> {
    let existing = state
        .orchestrator
        .generated_image_by_id(image_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "图片不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    let row = state
        .orchestrator
        .update_generated_image_record(
            image_id,
            UpdateImageRecord {
                prompt: input.prompt,
                image_type: input.image_type,
                chapter_id: input.chapter_id,
                clear_chapter_id: input.clear_chapter_id,
                panel_id: input.panel_id,
                clear_panel_id: input.clear_panel_id,
                ai_score: input.ai_score,
                clear_ai_score: input.clear_ai_score,
                is_favorite: input.is_favorite,
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(to_summary(&row))
}

pub async fn toggle_image_favorite(
    state: &AppState,
    user_id: i64,
    image_id: i64,
) -> Result<GeneratedImageSummary, String> {
    image_project_id_for_user(state, user_id, image_id).await?;
    let row = state
        .orchestrator
        .toggle_generated_image_favorite(image_id)
        .await
        .map_err(app_error_message)?;
    Ok(to_summary(&row))
}

pub async fn score_image(
    state: &AppState,
    user_id: i64,
    image_id: i64,
    score: f32,
) -> Result<GeneratedImageSummary, String> {
    image_project_id_for_user(state, user_id, image_id).await?;
    let row = state
        .orchestrator
        .update_generated_image_record(
            image_id,
            UpdateImageRecord {
                ai_score: Some(score),
                ..Default::default()
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(to_summary(&row))
}

async fn image_project_id_for_user(
    state: &AppState,
    user_id: i64,
    image_id: i64,
) -> Result<i64, String> {
    let image = state
        .orchestrator
        .generated_image_by_id(image_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "图片不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, image.project_id).await?;
    Ok(image.project_id)
}
