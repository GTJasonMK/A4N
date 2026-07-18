use super::super::contracts::{
    ProfileMaterialGroup, ProfileMaterialImageSummary, ProfileMaterialSource,
};
use super::super::prompts::metadata_text;
use crate::actions::image_generation::to_summary;
use a4n_core::models::ai::generated_image;
use std::collections::BTreeMap;

pub(in crate::actions::profile_materials) fn material_group(
    source: ProfileMaterialSource,
    images_by_profile: &mut BTreeMap<(String, i64), Vec<generated_image::Model>>,
    primary_by_profile: &BTreeMap<(String, i64), i64>,
    image_limit: usize,
) -> ProfileMaterialGroup {
    let key = (source.kind.to_string(), source.profile.id);
    let images = images_by_profile.remove(&key).unwrap_or_default();
    let primary_image_id = primary_by_profile.get(&key).copied();
    let newest_image_sort_key = images
        .iter()
        .map(|image| (image.created_at, image.id))
        .max();
    let has_favorite_image = images.iter().any(|image| image.is_favorite);
    let mut image_summaries = images
        .into_iter()
        .map(|image| {
            material_image_summary(
                &image,
                primary_image_id == Some(image.id),
                &source.content_hash,
            )
        })
        .collect::<Vec<_>>();
    image_summaries.sort_by(|a, b| {
        b.is_primary
            .cmp(&a.is_primary)
            .then_with(|| b.image.is_favorite.cmp(&a.image.is_favorite))
            .then_with(|| b.image.id.cmp(&a.image.id))
    });
    let primary_image = image_summaries
        .iter()
        .find(|image| image.is_primary)
        .cloned();
    let image_count = image_summaries.len();
    let material_state = material_state(&image_summaries);
    let images = image_summaries.into_iter().take(image_limit).collect();
    ProfileMaterialGroup {
        kind: source.kind.to_string(),
        profile: source.profile,
        material_state,
        current_profile_content_sha256: source.content_hash,
        image_count,
        primary_image_id,
        primary_image,
        images,
        newest_image_sort_key,
        has_favorite_image,
    }
}

fn material_image_summary(
    image: &generated_image::Model,
    is_primary: bool,
    current_profile_hash: &str,
) -> ProfileMaterialImageSummary {
    let metadata = image.metadata_json.as_ref();
    let profile_hash = metadata_text(metadata, "profile_content_sha256");
    let staleness = match profile_hash {
        Some(hash) if hash == current_profile_hash => "fresh",
        Some(_) => "stale",
        None => "unknown",
    }
    .to_string();
    ProfileMaterialImageSummary {
        image: to_summary(image),
        is_primary,
        staleness,
        style_template_id: metadata_text(metadata, "style_template_id").map(ToString::to_string),
        style_template_name: metadata_text(metadata, "style_template_name")
            .map(ToString::to_string),
    }
}

pub(in crate::actions::profile_materials) fn material_state(
    images: &[ProfileMaterialImageSummary],
) -> String {
    if images.is_empty() {
        return "missing".to_string();
    }
    if images.iter().any(|image| image.staleness == "stale") {
        return "stale".to_string();
    }
    if images.iter().all(|image| image.staleness == "unknown") {
        return "unknown".to_string();
    }
    "ready".to_string()
}
