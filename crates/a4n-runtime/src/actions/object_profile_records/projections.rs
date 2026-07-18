use super::ObjectProfileSummary;
use crate::{AppState, actions::profile_materials};
use a4n_core::models::asset::object_profile;
use a4n_orchestrator::profile_material::PROFILE_KIND_OBJECT;

pub(super) async fn to_summary(
    state: &AppState,
    profile: &object_profile::Model,
) -> Result<ObjectProfileSummary, String> {
    let generated_image_count = profile_materials::count_profile_material_images(
        state,
        profile.project_id,
        PROFILE_KIND_OBJECT,
        profile.id,
    )
    .await?;
    Ok(to_summary_with_image_count(profile, generated_image_count))
}

pub(super) fn to_summary_with_image_count(
    profile: &object_profile::Model,
    generated_image_count: i64,
) -> ObjectProfileSummary {
    ObjectProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        object_type: profile.object_type.clone(),
        importance: profile.importance.clone(),
        visual_description: profile.visual_description.clone(),
        material_and_state: profile.material_and_state.clone(),
        story_usage: profile.story_usage.clone(),
        visual_usage: profile.visual_usage.clone(),
        generation_usage: profile.generation_usage.clone(),
        location_context: profile.location_context.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
        generated_image_count,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    }
}
