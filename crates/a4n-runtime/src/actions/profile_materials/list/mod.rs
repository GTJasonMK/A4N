mod groups;
mod sorting;
mod sources;

use super::contracts::{
    IMAGE_TYPE_OBJECT, IMAGE_TYPE_SCENE, KIND_OBJECT, KIND_SCENE, ListProfileMaterialsInput,
    ProfileMaterialGroup,
};
use super::queries::material_identity;
use crate::AppState;
use crate::actions::access;
use a4n_core::models::ai::generated_image;
use std::collections::BTreeMap;

pub(in crate::actions::profile_materials) use groups::material_group;
#[cfg(test)]
pub(in crate::actions::profile_materials) use groups::material_state;
pub(in crate::actions::profile_materials) use sorting::{apply_group_filters, apply_group_sort};
pub(in crate::actions::profile_materials) use sources::{
    object_source_from_model, scene_source_from_model,
};

pub async fn list_profile_materials(
    state: &AppState,
    user_id: i64,
    input: ListProfileMaterialsInput,
) -> Result<Vec<ProfileMaterialGroup>, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let kinds = requested_kinds(input.kind.as_deref())?;
    let image_limit = input.image_limit_per_profile.unwrap_or(8).clamp(1, 32);

    let mut profiles = Vec::new();
    if kinds.contains(&KIND_SCENE) {
        profiles.extend(sources::scene_sources(state, input.project_id).await?);
    }
    if kinds.contains(&KIND_OBJECT) {
        profiles.extend(sources::object_sources(state, input.project_id).await?);
    }

    let material_rows = state
        .orchestrator
        .profile_material_list_rows(input.project_id, &[IMAGE_TYPE_SCENE, IMAGE_TYPE_OBJECT])
        .await
        .map_err(|error| error.to_string())?;
    let mut images_by_profile: BTreeMap<(String, i64), Vec<generated_image::Model>> =
        BTreeMap::new();
    for image in material_rows.images {
        if let Some((kind, profile_id)) = material_identity(&image) {
            images_by_profile
                .entry((kind.to_string(), profile_id))
                .or_default()
                .push(image);
        }
    }

    let primary_by_profile: BTreeMap<(String, i64), i64> = material_rows
        .primary_rows
        .into_iter()
        .map(|row| ((row.profile_kind, row.profile_id), row.primary_image_id))
        .collect();

    let mut groups = profiles
        .into_iter()
        .map(|source| {
            material_group(
                source,
                &mut images_by_profile,
                &primary_by_profile,
                image_limit,
            )
        })
        .collect::<Vec<_>>();

    apply_group_filters(&mut groups, input.query.as_deref(), input.filter.as_deref());
    apply_group_sort(&mut groups, input.sort.as_deref());
    Ok(groups)
}

pub(super) async fn read_single_profile_material_group(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<ProfileMaterialGroup, String> {
    let groups = list_profile_materials(
        state,
        user_id,
        ListProfileMaterialsInput {
            project_id,
            kind: Some(kind.to_string()),
            query: None,
            filter: None,
            sort: None,
            image_limit_per_profile: Some(32),
        },
    )
    .await?;
    groups
        .into_iter()
        .find(|group| group.profile.id == profile_id)
        .ok_or_else(|| "素材图已上传，但无法重新读取档案素材".to_string())
}

fn requested_kinds(kind: Option<&str>) -> Result<Vec<&'static str>, String> {
    match kind.map(str::trim).filter(|value| !value.is_empty()) {
        Some("scene") => Ok(vec![KIND_SCENE]),
        Some("object") => Ok(vec![KIND_OBJECT]),
        Some("all") | None => Ok(vec![KIND_SCENE, KIND_OBJECT]),
        Some(other) => Err(format!("不支持的素材档案类型: {other}")),
    }
}
