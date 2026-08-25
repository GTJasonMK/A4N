use a4n_core::blueprint_context::{
    blueprint_character_llm_context, blueprint_model_llm_context,
    blueprint_relationship_llm_context,
};
use a4n_core::models::{
    asset::object_profile,
    chapter::{chapter_outline, part_outline},
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship, novel_blueprint, novel_project},
    scene::scene_profile,
};
use serde_json::{Value, json};

pub(in crate::outline) fn outline_blueprint_context_json(
    project: &novel_project::Model,
    blueprint: Option<&novel_blueprint::Model>,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
    formal_materials: Value,
    chapters_per_part: Option<i32>,
) -> Value {
    let mut value = blueprint
        .map(blueprint_model_llm_context)
        .unwrap_or_else(|| {
            json!({
                "title": project.title,
                "project_type": project.project_type,
                "project_description": project.description,
            })
        });
    let object = value
        .as_object_mut()
        .expect("outline planning context is constructed as a JSON object");
    object.remove("total_chapters");
    object.insert(
        "blueprint_available".to_string(),
        json!(blueprint.is_some()),
    );
    object.insert("chapters_per_part".to_string(), json!(chapters_per_part));
    object.insert(
        "blueprint_chapters_per_part".to_string(),
        json!(blueprint.and_then(|value| value.chapters_per_part)),
    );
    object.insert(
        "characters".to_string(),
        json!(
            characters
                .iter()
                .map(blueprint_character_llm_context)
                .collect::<Vec<_>>()
        ),
    );
    object.insert(
        "relationships".to_string(),
        json!(
            relationships
                .iter()
                .map(blueprint_relationship_llm_context)
                .collect::<Vec<_>>()
        ),
    );
    object.insert("formal_materials".to_string(), formal_materials);
    value
}

pub(crate) fn outline_formal_materials_context_json(
    profiles: &[protagonist_profile::Model],
    objects: &[object_profile::Model],
    scenes: &[scene_profile::Model],
    foreshadowing: &[foreshadowing_index::Model],
) -> Value {
    json!({
        "character_profiles": profiles
            .iter()
            .map(character_profile_context_json)
            .collect::<Vec<_>>(),
        "objects": objects
            .iter()
            .map(object_profile_context_json)
            .collect::<Vec<_>>(),
        "scenes": scenes
            .iter()
            .map(scene_profile_context_json)
            .collect::<Vec<_>>(),
        "foreshadowing": foreshadowing
            .iter()
            .filter(|item| !matches!(item.status.as_str(), "resolved" | "abandoned"))
            .map(foreshadowing_context_json)
            .collect::<Vec<_>>(),
    })
}

pub(in crate::outline) fn has_outline_generation_source(
    blueprint_available: bool,
    project_description: &str,
    formal_materials: &Value,
    has_existing_planning: bool,
) -> bool {
    blueprint_available
        || !project_description.trim().is_empty()
        || has_existing_planning
        || ["character_profiles", "objects", "scenes", "foreshadowing"]
            .iter()
            .any(|key| {
                formal_materials
                    .get(key)
                    .and_then(Value::as_array)
                    .is_some_and(|items| !items.is_empty())
            })
}

pub(in crate::outline) fn part_context_json(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title.clone(),
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary.clone(),
        "theme": part.theme.clone(),
        "key_events": part.key_events.clone(),
        "character_focus": part.character_focus.clone(),
    })
}

pub(in crate::outline) fn minimal_blueprint_context(blueprint_context: &Value) -> Value {
    let Some(object) = blueprint_context.as_object() else {
        return blueprint_context.clone();
    };
    let mut minimal = serde_json::Map::new();
    for key in [
        "blueprint_available",
        "title",
        "project_type",
        "project_description",
        "genre",
        "style",
        "tone",
        "one_sentence_summary",
        "full_synopsis",
        "chapters_per_part",
        "blueprint_chapters_per_part",
        "formal_materials",
    ] {
        if let Some(value) = object.get(key) {
            minimal.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(minimal)
}

fn character_profile_context_json(profile: &protagonist_profile::Model) -> Value {
    let mut value = serde_json::Map::new();
    insert_text(&mut value, "name", &profile.name);
    insert_optional_text(&mut value, "gender", profile.gender.as_deref());
    insert_optional_text(&mut value, "age", profile.age.as_deref());
    insert_optional_text(&mut value, "appearance", profile.appearance.as_deref());
    insert_optional_text(&mut value, "background", profile.background.as_deref());
    insert_optional_text(&mut value, "personality", profile.personality.as_deref());
    insert_json(&mut value, "base_attributes", &profile.base_attributes);
    insert_json(&mut value, "current_state", &profile.current_state);
    if let Some(character_arc) = profile.character_arc.as_ref() {
        insert_json(&mut value, "character_arc", character_arc);
    }
    if let Some(voice_fingerprint) = profile.voice_fingerprint.as_ref() {
        insert_json(&mut value, "voice_fingerprint", voice_fingerprint);
    }
    if profile.last_synced_chapter > 0 {
        value.insert(
            "last_synced_chapter".to_string(),
            json!(profile.last_synced_chapter),
        );
    }
    Value::Object(value)
}

fn object_profile_context_json(profile: &object_profile::Model) -> Value {
    let mut value = serde_json::Map::new();
    insert_text(&mut value, "name", &profile.name);
    insert_json(&mut value, "aliases", &profile.aliases_json);
    insert_text(&mut value, "object_type", &profile.object_type);
    insert_text(&mut value, "importance", &profile.importance);
    insert_text(
        &mut value,
        "visual_description",
        &profile.visual_description,
    );
    insert_text(
        &mut value,
        "material_and_state",
        &profile.material_and_state,
    );
    insert_text(&mut value, "story_usage", &profile.story_usage);
    insert_optional_text(
        &mut value,
        "location_context",
        profile.location_context.as_deref(),
    );
    insert_confirmed_chapter_range(
        &mut value,
        profile.first_seen_chapter,
        profile.last_seen_chapter,
        &profile.evidence_json,
    );
    Value::Object(value)
}

fn scene_profile_context_json(profile: &scene_profile::Model) -> Value {
    let mut value = serde_json::Map::new();
    insert_text(&mut value, "name", &profile.name);
    insert_json(&mut value, "aliases", &profile.aliases_json);
    insert_text(&mut value, "scene_type", &profile.scene_type);
    insert_text(&mut value, "importance", &profile.importance);
    insert_text(&mut value, "base_description", &profile.base_description);
    insert_text(
        &mut value,
        "visual_description",
        &profile.visual_description,
    );
    insert_text(&mut value, "spatial_layout", &profile.spatial_layout);
    insert_text(&mut value, "atmosphere", &profile.atmosphere);
    insert_json(&mut value, "entry_points", &profile.entry_points_json);
    insert_json(&mut value, "rules", &profile.rules_json);
    insert_json(&mut value, "current_state", &profile.current_state_json);
    insert_confirmed_chapter_range(
        &mut value,
        profile.first_seen_chapter,
        profile.last_seen_chapter,
        &profile.evidence_json,
    );
    Value::Object(value)
}

fn foreshadowing_context_json(item: &foreshadowing_index::Model) -> Value {
    json!({
        "description": item.description,
        "setup_chapter": item.setup_chapter,
        "payoff_chapter": item.payoff_chapter,
        "status": item.status,
        "related_characters": item.related_characters,
        "importance": item.importance,
    })
}

fn insert_confirmed_chapter_range(
    object: &mut serde_json::Map<String, Value>,
    first_seen_chapter: i32,
    last_seen_chapter: i32,
    evidence: &Value,
) {
    if !is_meaningful_json(evidence) {
        return;
    }
    object.insert(
        "observed_chapter_range".to_string(),
        json!({
            "first": first_seen_chapter,
            "last": last_seen_chapter,
        }),
    );
}

fn insert_text(object: &mut serde_json::Map<String, Value>, key: &str, value: &str) {
    let value = value.trim();
    if !value.is_empty() {
        object.insert(key.to_string(), json!(value));
    }
}

fn insert_optional_text(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        insert_text(object, key, value);
    }
}

fn insert_json(object: &mut serde_json::Map<String, Value>, key: &str, value: &Value) {
    if is_meaningful_json(value) {
        object.insert(key.to_string(), value.clone());
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(values) => values.iter().any(is_meaningful_json),
        Value::Object(values) => values.values().any(is_meaningful_json),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

pub(in crate::outline) fn focused_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
        "theme": part.theme,
        "key_events": part.key_events,
    })
}

pub(in crate::outline) fn minimal_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
    })
}

pub(crate) fn chapter_outline_context_json(outline: &chapter_outline::Model) -> Value {
    json!({
        "chapter_number": outline.chapter_number,
        "title": outline.title.clone(),
        "summary": outline.summary.clone(),
        "beats": outline.beats.clone(),
        "key_scenes": outline.key_scenes.clone(),
        "foreshadowing_setup": outline.foreshadowing_setup.clone(),
        "foreshadowing_payoff": outline.foreshadowing_payoff.clone(),
        "characters_involved": outline.characters_involved.clone(),
        "pacing_score": outline.pacing_score,
        "word_count_target": outline.word_count_target,
    })
}
