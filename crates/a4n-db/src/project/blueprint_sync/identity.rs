use super::contracts::{BlueprintCharacterDraft, BlueprintRelationshipDraft};
use a4n_core::models::project::{blueprint_character, blueprint_relationship};

pub(crate) fn normalized_blueprint_identity(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(in crate::project::blueprint_sync) fn blueprint_relationship_identity_from_keys(
    relationship_type: &str,
    character_a: &str,
    character_b: &str,
) -> String {
    format!("{relationship_type}|{character_a}|{character_b}")
}

pub(in crate::project::blueprint_sync) fn blueprint_relationship_model_identity(
    relationship: &blueprint_relationship::Model,
) -> String {
    format!(
        "{}|{}|{}",
        normalized_blueprint_identity(&relationship.relationship_type),
        normalized_blueprint_identity(&relationship.character_a),
        normalized_blueprint_identity(&relationship.character_b)
    )
}

pub(in crate::project::blueprint_sync) fn blueprint_character_matches(
    existing: &blueprint_character::Model,
    draft: &BlueprintCharacterDraft,
) -> bool {
    existing.name == draft.name
        && existing.character_type == draft.character_type
        && existing.gender == draft.gender
        && existing.age == draft.age
        && existing.appearance == draft.appearance
        && existing.personality == draft.personality
        && existing.background == draft.background
        && existing.motivation == draft.motivation
        && existing.arc_summary == draft.arc_summary
        && existing.relationship_to_protagonist == draft.relationship_to_protagonist
        && existing.arc_stages == draft.arc_stages
}

pub(in crate::project::blueprint_sync) fn blueprint_relationship_matches(
    existing: &blueprint_relationship::Model,
    draft: &BlueprintRelationshipDraft,
) -> bool {
    existing.relationship_type == draft.relationship_type
        && existing.character_a == draft.character_a
        && existing.character_b == draft.character_b
        && existing.description == draft.description
        && existing.strength == draft.strength
        && existing.evolution == draft.evolution
}
