use super::ObjectProfileInput;
use a4n_orchestrator::asset::ObjectProfileDraft;

pub(super) fn into_draft(input: ObjectProfileInput) -> ObjectProfileDraft {
    ObjectProfileDraft {
        name: input.name,
        aliases: input.aliases,
        object_type: input.object_type,
        importance: input.importance,
        visual_description: input.visual_description,
        material_and_state: input.material_and_state,
        story_usage: input.story_usage,
        visual_usage: input.visual_usage,
        generation_usage: input.generation_usage,
        location_context: input.location_context,
        first_seen_chapter: input.first_seen_chapter,
        last_seen_chapter: input.last_seen_chapter,
        evidence: input.evidence,
        image_prompt_seed: input.image_prompt_seed,
    }
}
