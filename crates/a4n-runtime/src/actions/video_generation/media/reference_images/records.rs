use crate::AppState;
use crate::actions::access;
use a4n_core::models::ai::generated_image;
use serde_json::json;
use std::collections::{HashMap, HashSet};

pub(super) async fn validated_reference_images_for_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<Vec<generated_image::Model>, String> {
    let images = state
        .orchestrator
        .validated_video_reference_images(project_id, chapter_id, Some(&json!(ids)))
        .await
        .map_err(|error| error.to_string())?;
    let project_ids = unique_project_ids_from_images(&images);
    verify_reference_image_project_editors(state, user_id, project_ids).await?;
    Ok(images)
}

pub(super) async fn load_reference_images_by_id(
    state: &AppState,
    ids: &[i64],
) -> Result<HashMap<i64, generated_image::Model>, String> {
    let rows = state
        .orchestrator
        .generated_images_by_ids(ids)
        .await
        .map_err(|error| error.to_string())?;
    Ok(rows.into_iter().map(|image| (image.id, image)).collect())
}

pub(in crate::actions::video_generation::media) fn reference_image_project_ids_in_reference_order(
    ids: &[i64],
    images_by_id: &HashMap<i64, generated_image::Model>,
) -> Vec<i64> {
    let mut seen = HashSet::new();
    let mut project_ids = Vec::new();
    for id in ids {
        let Some(image) = images_by_id.get(id) else {
            continue;
        };
        if seen.insert(image.project_id) {
            project_ids.push(image.project_id);
        }
    }
    project_ids
}

fn unique_project_ids_from_images(images: &[generated_image::Model]) -> Vec<i64> {
    let mut seen = HashSet::new();
    let mut project_ids = Vec::new();
    for image in images {
        if seen.insert(image.project_id) {
            project_ids.push(image.project_id);
        }
    }
    project_ids
}

pub(super) async fn verify_reference_image_project_editors(
    state: &AppState,
    user_id: i64,
    project_ids: Vec<i64>,
) -> Result<(), String> {
    for project_id in project_ids {
        access::verify_project_editor_string(state, user_id, project_id).await?;
    }
    Ok(())
}
