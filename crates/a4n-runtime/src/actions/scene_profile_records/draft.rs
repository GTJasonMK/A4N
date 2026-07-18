use super::SceneProfileInput;
use a4n_orchestrator::scene_profile::SceneProfileDraft;
use serde_json::Value;

impl SceneProfileInput {
    pub(super) fn into_draft(self) -> SceneProfileDraft {
        SceneProfileDraft {
            scene_ref: self.name.clone(),
            scene_profile_id: None,
            name: self.name,
            aliases: self.aliases,
            scene_type: self.scene_type,
            importance: self.importance,
            parent_scene_id: self.parent_scene_id,
            base_description: self.base_description,
            visual_description: self.visual_description,
            spatial_layout: self.spatial_layout,
            atmosphere: self.atmosphere,
            entry_points: self
                .entry_points_json
                .unwrap_or_else(|| Value::Array(Vec::new())),
            rules: self.rules_json.unwrap_or_else(|| Value::Array(Vec::new())),
            first_seen_chapter: self.first_seen_chapter,
            last_seen_chapter: self.last_seen_chapter,
            evidence: self.evidence,
            image_prompt_seed: self.image_prompt_seed,
        }
    }
}
