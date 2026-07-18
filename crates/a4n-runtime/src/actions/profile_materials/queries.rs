use super::contracts::{IMAGE_TYPE_OBJECT, IMAGE_TYPE_SCENE, KIND_SCENE};
use crate::AppState;
use a4n_core::models::ai::generated_image;
use a4n_orchestrator::profile_material::generated_image_profile_material_identity;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn material_identity(image: &generated_image::Model) -> Option<(&'static str, i64)> {
    generated_image_profile_material_identity(image)
}

pub(super) fn normalize_kind(kind: &str) -> Result<&'static str, String> {
    match kind.trim() {
        "scene" => Ok(KIND_SCENE),
        "object" => Ok(super::contracts::KIND_OBJECT),
        other => Err(format!("不支持的档案素材类型: {other}")),
    }
}
fn image_type(kind: &str) -> &'static str {
    if kind == KIND_SCENE {
        IMAGE_TYPE_SCENE
    } else {
        IMAGE_TYPE_OBJECT
    }
}

pub(crate) async fn count_profile_material_images(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<i64, String> {
    let kind = normalize_kind(kind)?;
    let images = state
        .orchestrator
        .profile_material_images_by_type(project_id, image_type(kind))
        .await
        .map_err(|error| error.to_string())?;
    Ok(images
        .into_iter()
        .filter(|image| material_identity(image) == Some((kind, profile_id)))
        .count() as i64)
}
pub(crate) async fn count_profile_material_images_by_profile(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_ids: &[i64],
) -> Result<BTreeMap<i64, i64>, String> {
    let kind = normalize_kind(kind)?;
    if profile_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let requested = profile_ids.iter().copied().collect::<BTreeSet<_>>();
    let images = state
        .orchestrator
        .profile_material_images_by_type(project_id, image_type(kind))
        .await
        .map_err(|error| error.to_string())?;
    let mut counts = BTreeMap::new();
    for image in images {
        if let Some((image_kind, id)) = material_identity(&image) {
            if image_kind == kind && requested.contains(&id) {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
    }
    Ok(counts)
}
pub(crate) async fn list_profile_material_image_ids(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<Vec<i64>, String> {
    let kind = normalize_kind(kind)?;
    let images = state
        .orchestrator
        .profile_material_images_by_type(project_id, image_type(kind))
        .await
        .map_err(|error| error.to_string())?;
    Ok(images
        .into_iter()
        .filter(|image| material_identity(image) == Some((kind, profile_id)))
        .map(|image| image.id)
        .collect())
}
