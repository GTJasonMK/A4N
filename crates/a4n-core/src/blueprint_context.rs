use crate::models::project::{blueprint_character, blueprint_relationship, novel_blueprint};
use serde_json::{Value, json};

pub fn blueprint_llm_context(
    blueprint: &novel_blueprint::Model,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
) -> Value {
    json!({
        "blueprint": blueprint_model_llm_context(blueprint),
        "characters": characters
            .iter()
            .map(blueprint_character_llm_context)
            .collect::<Vec<_>>(),
        "relationships": relationships
            .iter()
            .map(blueprint_relationship_llm_context)
            .collect::<Vec<_>>(),
    })
}

pub fn blueprint_model_llm_context(blueprint: &novel_blueprint::Model) -> Value {
    json!({
        "title": blueprint.title,
        "genre": blueprint.genre,
        "style": blueprint.style,
        "tone": blueprint.tone,
        "target_audience": blueprint.target_audience,
        "one_sentence_summary": blueprint.one_sentence_summary,
        "full_synopsis": blueprint.full_synopsis,
        "world_setting": blueprint.world_setting,
        "needs_part_outlines": blueprint.needs_part_outlines,
        "total_chapters": blueprint.total_chapters,
        "chapters_per_part": blueprint.chapters_per_part,
    })
}

pub fn blueprint_character_llm_context(character: &blueprint_character::Model) -> Value {
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
}

pub fn blueprint_relationship_llm_context(relationship: &blueprint_relationship::Model) -> Value {
    json!({
        "relationship_type": relationship.relationship_type,
        "character_a": relationship.character_a,
        "character_b": relationship.character_b,
        "description": relationship.description,
        "strength": relationship.strength,
        "evolution": relationship.evolution,
    })
}

#[cfg(test)]
mod tests;
