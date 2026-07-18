use crate::state::AppState;
use a4n_core::models::asset::object_profile;
use a4n_core::models::chapter::chapter;
use a4n_orchestrator::asset::ObjectProfileUpsertRecordResult;
use serde_json::{Value, json};

pub(super) async fn object_discovery_result(
    _state: &AppState,
    chapters: Vec<chapter::Model>,
    discovered_count: usize,
    result: ObjectProfileUpsertRecordResult,
) -> Result<Value, String> {
    let mut created = Vec::with_capacity(result.created.len());
    for profile in &result.created {
        created.push(object_profile_result(profile));
    }
    let mut updated = Vec::with_capacity(result.updated.len());
    for profile in &result.updated {
        updated.push(object_profile_result(profile));
    }
    Ok(json!({
        "status": "discovered",
        "chapters_analyzed": chapters.len(),
        "chapter_numbers": chapters.iter().map(|chapter| chapter.chapter_number).collect::<Vec<_>>(),
        "discovered_count": discovered_count,
        "created": created,
        "updated": updated}))
}

fn object_profile_result(profile: &object_profile::Model) -> Value {
    json!({
        "id": profile.id,
        "project_id": profile.project_id,
        "name": profile.name,
        "normalized_name": profile.normalized_name,
        "aliases_json": profile.aliases_json,
        "object_type": profile.object_type,
        "importance": profile.importance,
        "visual_description": profile.visual_description,
        "material_and_state": profile.material_and_state,
        "story_usage": profile.story_usage,
        "visual_usage": profile.visual_usage,
        "generation_usage": profile.generation_usage,
        "location_context": profile.location_context,
        "first_seen_chapter": profile.first_seen_chapter,
        "last_seen_chapter": profile.last_seen_chapter,
        "evidence_json": profile.evidence_json,
        "image_prompt_seed": profile.image_prompt_seed,
        "created_at": profile.created_at.to_string(),
        "updated_at": profile.updated_at.to_string()})
}
