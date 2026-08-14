use crate::{
    AppState,
    actions::{
        app_error_message,
        image_generation::{GeneratedImageSummary, to_summary},
    },
};
use a4n_orchestrator::video::UpdateGeneratedImageRecordInput as UpdateImageRecord;

use super::UpdateGeneratedImageInput;

pub async fn update_image(
    state: &AppState,
    user_id: i64,
    input: UpdateGeneratedImageInput,
) -> Result<GeneratedImageSummary, String> {
    if input.is_empty() {
        return Err("图片更新至少需要一个字段".to_string());
    }
    let image_id = input.image_id;
    let clear_chapter_id = input.chapter_id.as_ref().is_some_and(Option::is_none);
    let clear_panel_id = input.panel_id.as_ref().is_some_and(Option::is_none);
    let clear_ai_score = input.ai_score.as_ref().is_some_and(Option::is_none);
    let row = state
        .orchestrator()
        .update_generated_image_for_user_validated(
            user_id,
            image_id,
            UpdateImageRecord {
                prompt: input.prompt,
                image_type: input.image_type,
                chapter_id: input.chapter_id.flatten(),
                clear_chapter_id,
                panel_id: input.panel_id.flatten(),
                clear_panel_id,
                ai_score: input.ai_score.flatten(),
                clear_ai_score,
                is_favorite: input.is_favorite,
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(to_summary(&row))
}
