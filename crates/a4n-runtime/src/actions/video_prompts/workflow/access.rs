use crate::{AppState, actions::access};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(in crate::actions::video_prompts) async fn verify_video_reference_image_project_access_for_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids: &[i64],
) -> Result<(), String> {
    if reference_image_ids.is_empty() {
        return Ok(());
    }
    let ids = json!(reference_image_ids);
    verify_video_reference_image_project_access(state, user_id, project_id, chapter_id, Some(&ids))
        .await
}

pub(in crate::actions::video_prompts) async fn verify_video_reference_image_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: Option<&Value>,
) -> Result<(), String> {
    let images = state
        .orchestrator
        .validated_video_reference_images(project_id, chapter_id, reference_image_ids_json)
        .await
        .map_err(|e| e.to_string())?;
    let project_ids = images
        .into_iter()
        .map(|image| image.project_id)
        .collect::<BTreeSet<_>>();
    for image_project_id in project_ids {
        access::verify_project_editor_string(state, user_id, image_project_id).await?;
    }
    Ok(())
}
