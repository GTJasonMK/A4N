use super::super::contracts::{
    KIND_OBJECT, KIND_SCENE, ProfileMaterialProfileSummary, ProfileMaterialSource,
};
use super::super::prompts::{
    first_non_empty, first_non_empty_opt, object_profile_content, scene_profile_content,
};
use crate::AppState;
use a4n_core::models::{asset::object_profile, scene::scene_profile};
use a4n_core::source_spans::source_text_hash as source_content_sha256;

pub(super) async fn scene_sources(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<ProfileMaterialSource>, String> {
    let profiles = state
        .orchestrator
        .scene_profiles_by_project(project_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(profiles.into_iter().map(scene_source_from_model).collect())
}

pub(super) async fn object_sources(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<ProfileMaterialSource>, String> {
    let profiles = state
        .orchestrator
        .object_profiles_by_project(project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(profiles.into_iter().map(object_source_from_model).collect())
}

pub(in crate::actions::profile_materials) fn scene_source_from_model(
    profile: scene_profile::Model,
) -> ProfileMaterialSource {
    let content_hash = source_content_sha256(&scene_profile_content(&profile));
    ProfileMaterialSource {
        kind: KIND_SCENE,
        profile: ProfileMaterialProfileSummary {
            id: profile.id,
            project_id: profile.project_id,
            name: profile.name,
            category: profile.scene_type,
            importance: profile.importance,
            first_seen_chapter: profile.first_seen_chapter,
            last_seen_chapter: profile.last_seen_chapter,
            primary_text: first_non_empty(&[
                &profile.visual_description,
                &profile.base_description,
                &profile.atmosphere,
            ]),
            secondary_text: first_non_empty(&[&profile.spatial_layout, &profile.atmosphere]),
            updated_at: Some(profile.updated_at.to_string()),
        },
        content_hash,
    }
}

pub(in crate::actions::profile_materials) fn object_source_from_model(
    profile: object_profile::Model,
) -> ProfileMaterialSource {
    let content_hash = source_content_sha256(&object_profile_content(&profile));
    ProfileMaterialSource {
        kind: KIND_OBJECT,
        profile: ProfileMaterialProfileSummary {
            id: profile.id,
            project_id: profile.project_id,
            name: profile.name,
            category: profile.object_type,
            importance: profile.importance,
            first_seen_chapter: profile.first_seen_chapter,
            last_seen_chapter: profile.last_seen_chapter,
            primary_text: first_non_empty(&[
                &profile.visual_description,
                &profile.generation_usage,
                &profile.visual_usage,
                &profile.story_usage,
                &profile.material_and_state,
            ]),
            secondary_text: first_non_empty_opt(&[
                Some(profile.material_and_state.as_str()),
                Some(profile.visual_usage.as_str()),
                profile.location_context.as_deref(),
            ]),
            updated_at: Some(profile.updated_at.to_string()),
        },
        content_hash,
    }
}
