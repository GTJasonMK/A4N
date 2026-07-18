use crate::{
    AppState,
    actions::{
        access, app_error_message,
        image_generation::{GeneratedImageSummary, to_summary},
    },
};
use std::collections::{HashMap, HashSet};

use super::{
    ListVideoReferenceImagesInput,
    rules::{
        image_type_rank, is_video_reference_image_candidate,
        resolve_video_reference_image_project_ids, video_reference_scope_rank,
    },
};

pub async fn list_images_by_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<GeneratedImageSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let rows = state
        .orchestrator
        .generated_images_for_project(project_id)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_images_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedImageSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let rows = state
        .orchestrator
        .generated_images_for_valid_chapter(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_video_reference_images(
    state: &AppState,
    user_id: i64,
    input: ListVideoReferenceImagesInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let allowed_project_ids = state
        .orchestrator
        .video_asset_project_ids(input.project_id)
        .await
        .map_err(app_error_message)?;
    let image_project_ids =
        resolve_video_reference_image_project_ids(input.image_project_ids, &allowed_project_ids)?;
    access::verify_projects_editor_string(state, user_id, &image_project_ids).await?;
    let primary_image_ids = state
        .orchestrator
        .primary_material_image_ids_for_projects(&image_project_ids)
        .await
        .map_err(crate::actions::app_error_message)?
        .into_iter()
        .collect::<HashSet<_>>();
    let project_images = state
        .orchestrator
        .video_reference_candidate_images(&image_project_ids, input.chapter_id)
        .await
        .map_err(app_error_message)?;
    let mut by_id = HashMap::new();
    for image in project_images {
        if is_video_reference_image_candidate(&image, input.chapter_id) {
            by_id.insert(image.id, image);
        }
    }
    let mut images = by_id.into_values().collect::<Vec<_>>();
    images.sort_by(|a, b| {
        video_reference_scope_rank(a, input.chapter_id)
            .cmp(&video_reference_scope_rank(b, input.chapter_id))
            .then_with(|| {
                primary_image_ids
                    .contains(&b.id)
                    .cmp(&primary_image_ids.contains(&a.id))
            })
            .then_with(|| b.is_favorite.cmp(&a.is_favorite))
            .then_with(|| image_type_rank(&a.image_type).cmp(&image_type_rank(&b.image_type)))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(images.iter().map(to_summary).collect())
}
