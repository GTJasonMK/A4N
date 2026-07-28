use super::SceneProfileInput;
use a4n_orchestrator::scene_profile::SceneProfileDraft;
use serde_json::Value;

pub(super) fn into_draft(input: SceneProfileInput) -> SceneProfileDraft {
    SceneProfileDraft {
        scene_ref: input.name.clone(),
        scene_profile_id: None,
        name: input.name,
        aliases: input.aliases,
        scene_type: input.scene_type,
        importance: input.importance,
        parent_scene_id: input.parent_scene_id,
        base_description: input.base_description,
        visual_description: input.visual_description,
        spatial_layout: input.spatial_layout,
        atmosphere: input.atmosphere,
        entry_points: input
            .entry_points_json
            .unwrap_or_else(|| Value::Array(Vec::new())),
        rules: input.rules_json.unwrap_or_else(|| Value::Array(Vec::new())),
        first_seen_chapter: input.first_seen_chapter,
        last_seen_chapter: input.last_seen_chapter,
        evidence: input.evidence,
        image_prompt_seed: input.image_prompt_seed,
    }
}
