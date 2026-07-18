use a4n_db::import_reverse::ReverseBlueprintContextRows;
use serde_json::{Value, json};

pub(super) fn existing_blueprint_projection(rows: &ReverseBlueprintContextRows) -> Option<Value> {
    rows.existing_blueprint.as_ref().map(|blueprint| {
        json!({
            "title": blueprint.title,
            "genre": blueprint.genre,
            "style": blueprint.style,
            "tone": blueprint.tone,
            "target_audience": blueprint.target_audience,
            "one_sentence_summary": blueprint.one_sentence_summary,
            "full_synopsis": blueprint.full_synopsis,
            "world_setting": blueprint.world_setting,
        })
    })
}

pub(super) fn existing_character_projections(rows: &ReverseBlueprintContextRows) -> Vec<Value> {
    rows.blueprint_characters
        .iter()
        .map(|character| {
            json!({
                "name": character.name,
                "character_type": character.character_type,
                "gender": character.gender,
                "age": character.age,
                "appearance": character.appearance,
                "personality": character.personality,
                "background": character.background,
                "motivation": character.motivation,
                "arc_summary": character.arc_summary,
                "relationship_to_protagonist": character.relationship_to_protagonist,
                "arc_stages": character.arc_stages,
            })
        })
        .collect()
}

pub(super) fn protagonist_projections(rows: &ReverseBlueprintContextRows) -> Vec<Value> {
    rows.protagonist_profiles
        .iter()
        .map(|profile| {
            json!({
                "name": profile.name,
                "gender": profile.gender,
                "age": profile.age,
                "base_attributes": profile.base_attributes,
                "current_state": profile.current_state,
                "character_arc": profile.character_arc,
                "appearance": profile.appearance,
                "background": profile.background,
                "personality": profile.personality,
            })
        })
        .collect()
}
