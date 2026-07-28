use a4n_core::models::chapter::{chapter, chapter_outline, chapter_scene};
use serde_json::{Value, json};

fn scene_context_value(scene: &chapter_scene::Model) -> Value {
    json!({
        "scene_number": scene.scene_number,
        "title": scene.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": scene.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "scene_type": scene.scene_type,
        "characters_present": scene.characters_present.clone(),
        "location": scene.location.as_deref().map(str::trim).filter(|location| !location.is_empty()),
        "timeline_position": scene.timeline_position.as_deref().map(str::trim).filter(|position| !position.is_empty()),
        "content": scene.content,
    })
}

fn outline_context_value(outline: &chapter_outline::Model) -> Value {
    json!({
        "chapter_number": outline.chapter_number,
        "title": outline.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": outline.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "beats": outline.beats.clone(),
        "key_scenes": outline.key_scenes.clone(),
        "pacing_score": outline.pacing_score,
        "foreshadowing_setup": outline.foreshadowing_setup.clone(),
        "foreshadowing_payoff": outline.foreshadowing_payoff.clone(),
        "characters_involved": outline.characters_involved.clone(),
        "word_count_target": outline.word_count_target,
    })
}

pub(super) fn scene_generation_stable_context(
    chapter_number: i32,
    title: &str,
    summary: &str,
    generation_context: Option<&Value>,
) -> Value {
    json!({
        "chapter": {
            "chapter_number": chapter_number,
            "title": title,
            "summary": summary,
        },
        "available_context": generation_context.cloned().unwrap_or_else(|| json!({})),
    })
}

pub(super) fn build_scene_generation_context(
    chapter: &chapter::Model,
    outline: Option<&chapter_outline::Model>,
    scenes: &[chapter_scene::Model],
    target_scene_number: i32,
) -> Value {
    let mut object = serde_json::Map::new();
    if chapter.has_content() {
        object.insert("chapter_content".to_string(), json!(chapter.content));
    }
    if let Some(outline) = outline {
        object.insert(
            "chapter_outline".to_string(),
            outline_context_value(outline),
        );
    }

    let existing_scenes = scenes
        .iter()
        .filter(|scene| scene.scene_number != target_scene_number)
        .map(scene_context_value)
        .collect::<Vec<_>>();
    if !existing_scenes.is_empty() {
        object.insert("existing_scenes".to_string(), Value::Array(existing_scenes));
    }
    if let Some(target_scene) = scenes
        .iter()
        .find(|scene| scene.scene_number == target_scene_number)
    {
        object.insert(
            "existing_target_scene".to_string(),
            scene_context_value(target_scene),
        );
    }

    Value::Object(object)
}
