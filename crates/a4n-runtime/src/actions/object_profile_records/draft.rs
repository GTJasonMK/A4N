use super::ObjectProfileInput;
use a4n_orchestrator::asset::ObjectProfileDraft;

impl ObjectProfileInput {
    pub(super) fn into_draft(self) -> ObjectProfileDraft {
        ObjectProfileDraft {
            name: self.name,
            aliases: self.aliases,
            object_type: self.object_type,
            importance: self.importance,
            visual_description: self.visual_description,
            material_and_state: self.material_and_state,
            story_usage: self.story_usage,
            visual_usage: self.visual_usage,
            generation_usage: self.generation_usage,
            location_context: self.location_context,
            first_seen_chapter: self.first_seen_chapter,
            last_seen_chapter: self.last_seen_chapter,
            evidence: self.evidence,
            image_prompt_seed: self.image_prompt_seed,
        }
    }
}
