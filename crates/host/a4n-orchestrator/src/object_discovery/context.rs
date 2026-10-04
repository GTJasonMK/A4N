use a4n_domain::source_spans::text_anchor_prompt_text;
use a4n_domain::{
    chapter::ChapterRecord, object_profile::ObjectProfileRecord, text_anchor::TextAnchorRecord,
};
use a4n_error::AppError;
use serde_json::{Value, json};

pub(super) fn object_discovery_stable_context(
    existing_profiles: &[Value],
    chapters: &[ChapterRecord],
    text_anchors: &[TextAnchorRecord],
) -> Result<Value, AppError> {
    let chapter_data = chapters.iter().map(|chapter| {
        Ok(json!({
            "chapter_number": chapter.chapter_number,
            "content": text_anchor_prompt_text(chapter, text_anchors).map_err(AppError::InvalidArgument)?,
        }))
    }).collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "existing_object_profiles": existing_profiles,
        "chapters": chapter_data,
    }))
}

pub(super) fn existing_object_profile_for_prompt(profile: &ObjectProfileRecord) -> Value {
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
