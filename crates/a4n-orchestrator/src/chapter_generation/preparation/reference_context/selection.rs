use std::collections::HashSet;

use super::json::extract_character_names_from_value;
use a4n_db_models::models::{
    chapter::chapter_outline,
    character::foreshadowing_index,
    project::{blueprint_character, blueprint_relationship},
};

pub(in crate::chapter_generation::preparation) fn select_generation_characters<'a>(
    characters: &'a [blueprint_character::Model],
    involved_names: &[String],
) -> Vec<&'a blueprint_character::Model> {
    let selected = if involved_names.is_empty() {
        characters
            .iter()
            .filter(|character| {
                matches!(
                    character.character_type.as_str(),
                    "protagonist" | "antagonist" | "supporting"
                )
            })
            .collect::<Vec<_>>()
    } else {
        let involved_names = involved_names
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        characters
            .iter()
            .filter(|character| involved_names.contains(character.name.as_str()))
            .collect::<Vec<_>>()
    };

    if selected.is_empty() {
        characters.iter().collect()
    } else {
        selected
    }
}

pub(super) fn select_generation_relationships<'a>(
    relationships: &'a [blueprint_relationship::Model],
    involved_names: &[String],
) -> Vec<&'a blueprint_relationship::Model> {
    if involved_names.is_empty() {
        return relationships.iter().collect();
    }
    let involved_names = involved_names
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    relationships
        .iter()
        .filter(|relationship| {
            involved_names.contains(relationship.character_a.as_str())
                || involved_names.contains(relationship.character_b.as_str())
        })
        .collect()
}

pub(in crate::chapter_generation::preparation) fn select_generation_foreshadowing<'a>(
    foreshadowing: &'a [foreshadowing_index::Model],
    chapter_number: i32,
    involved_names: &[String],
) -> Vec<&'a foreshadowing_index::Model> {
    let involved_name_set = involved_names
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    foreshadowing
        .iter()
        .filter(|item| {
            let relevant_time =
                item.setup_chapter < chapter_number || item.payoff_chapter == Some(chapter_number);
            let unresolved = match item.payoff_chapter {
                Some(payoff) => payoff >= chapter_number,
                None => true,
            };
            relevant_time
                && unresolved
                && (involved_names.is_empty()
                    || item.related_characters.as_ref().is_some_and(|value| {
                        let related = extract_character_names_from_value(value);
                        related
                            .iter()
                            .any(|name| involved_name_set.contains(name.as_str()))
                    }))
        })
        .collect()
}

pub fn outline_character_names(outline: Option<&chapter_outline::Model>) -> Vec<String> {
    match outline.and_then(|outline| outline.characters_involved.as_ref()) {
        Some(value) => extract_character_names_from_value(value),
        None => Vec::new(),
    }
}
