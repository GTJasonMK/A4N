use super::SceneProfileSummary;
use crate::actions::profile_materials;
use a4n_core::models::scene::scene_profile;
use a4n_domain::profile_material::ProfileMaterialKind;
use std::collections::BTreeMap;

pub(super) async fn to_summary(
    scene_profile: &a4n_orchestrator::OrchestratorContext,
    profile_material: &a4n_orchestrator::OrchestratorContext,
    profile: &scene_profile::Model,
) -> Result<SceneProfileSummary, String> {
    let occurrence_count = occurrence_count(scene_profile, profile).await?;
    let generated_image_count = profile_materials::count_profile_material_images(
        profile_material,
        profile.project_id,
        ProfileMaterialKind::Scene,
        profile.id,
    )
    .await?;
    Ok(to_summary_with_counts(
        profile,
        occurrence_count,
        generated_image_count,
    ))
}

pub(super) fn to_summary_with_counts(
    profile: &scene_profile::Model,
    occurrence_count: u64,
    generated_image_count: i64,
) -> SceneProfileSummary {
    SceneProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        ownership_kind: profile.ownership_kind.clone(),
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        scene_type: profile.scene_type.clone(),
        importance: profile.importance.clone(),
        parent_scene_id: profile.parent_scene_id,
        base_description: profile.base_description.clone(),
        visual_description: profile.visual_description.clone(),
        spatial_layout: profile.spatial_layout.clone(),
        atmosphere: profile.atmosphere.clone(),
        entry_points_json: profile.entry_points_json.clone(),
        rules_json: profile.rules_json.clone(),
        current_state_json: profile.current_state_json.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
        occurrence_count,
        generated_image_count,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    }
}

async fn occurrence_count(
    scene_profile: &a4n_orchestrator::OrchestratorContext,
    profile: &scene_profile::Model,
) -> Result<u64, String> {
    scene_profile
        .scene_occurrence_count(profile.project_id, profile.id)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn occurrence_counts_by_profile(
    scene_profile: &a4n_orchestrator::OrchestratorContext,
    project_id: i64,
    profile_ids: &[i64],
) -> Result<BTreeMap<i64, u64>, String> {
    if profile_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    scene_profile
        .scene_occurrence_counts_by_profile(project_id, profile_ids)
        .await
        .map_err(|error| error.to_string())
}
