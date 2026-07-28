use a4n_core::blueprint_context::{
    blueprint_character_llm_context, blueprint_model_llm_context,
    blueprint_relationship_llm_context,
};
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_core::models::project::{blueprint_character, blueprint_relationship, novel_blueprint};
use serde_json::{Value, json};

pub fn outline_blueprint_context_json(
    blueprint: &novel_blueprint::Model,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
    chapters_per_part: Option<i32>,
) -> Value {
    let mut value = blueprint_model_llm_context(blueprint);
    let object = value
        .as_object_mut()
        .expect("blueprint_model_llm_context returns a JSON object");
    object.remove("total_chapters");
    object.insert("chapters_per_part".to_string(), json!(chapters_per_part));
    object.insert(
        "blueprint_chapters_per_part".to_string(),
        json!(blueprint.chapters_per_part),
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
    value
}

pub fn part_context_json(part: &part_outline::Model) -> Value {
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

pub fn chapter_outline_context_json(outline: &chapter_outline::Model) -> Value {
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
