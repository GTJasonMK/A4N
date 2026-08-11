use a4n_core::blueprint_context::{
    blueprint_character_llm_context, blueprint_model_llm_context,
    blueprint_relationship_llm_context,
};
use a4n_db::import_reverse::ReverseBlueprintContextRows;
use serde_json::{Value, json};

pub(super) fn existing_blueprint_projection(rows: &ReverseBlueprintContextRows) -> Option<Value> {
    rows.existing_blueprint
        .as_ref()
        .map(blueprint_model_llm_context)
}

pub(super) fn existing_character_projections(rows: &ReverseBlueprintContextRows) -> Vec<Value> {
    rows.blueprint_characters
        .iter()
        .map(blueprint_character_llm_context)
        .collect()
}

pub(super) fn existing_relationship_projections(rows: &ReverseBlueprintContextRows) -> Vec<Value> {
    rows.blueprint_relationships
        .iter()
        .map(blueprint_relationship_llm_context)
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
