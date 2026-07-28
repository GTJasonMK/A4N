use a4n_core::{
    models::{asset::object_profile, chapter::chapter, project::text_anchor},
    source_spans::text_anchor_prompt_values,
};
use serde_json::{Value, json};

pub(super) fn object_discovery_stable_context(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Value {
    json!({
        "existing_object_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors),
    })
}

pub(super) fn object_discovery_chapter_context(chapter: &chapter::Model) -> Value {
    json!({
        "chapter_number": chapter.chapter_number,
        "chapter_title": chapter.title,
        "content": chapter.content,
        "analysis_data": chapter.analysis_data,
    })
}

pub(super) fn existing_object_profile_for_prompt(profile: &object_profile::Model) -> Value {
    json!({
        "name": profile.name,
        "aliases": profile.aliases_json,
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
    })
}
