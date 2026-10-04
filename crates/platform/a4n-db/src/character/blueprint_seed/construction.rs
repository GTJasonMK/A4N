use super::contracts::BlueprintProfileSeed;
use a4n_domain::project::{BlueprintCharacterRecord, BlueprintRelationshipRecord};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

pub(super) fn profile_seed_from_blueprint_character(
    character: &BlueprintCharacterRecord,
    relationships: &[(String, String)],
) -> BlueprintProfileSeed {
    let mut explicit = Map::new();
    seed_insert_text(
        &mut explicit,
        "character_type",
        Some(character.character_type.as_str()),
    );
    seed_insert_text(&mut explicit, "gender", character.gender.as_deref());
    seed_insert_text(&mut explicit, "age", character.age.as_deref());
    seed_insert_text(&mut explicit, "appearance", character.appearance.as_deref());
    seed_insert_text(&mut explicit, "background", character.background.as_deref());
    seed_insert_text(&mut explicit, "motivation", character.motivation.as_deref());
    seed_insert_text(
        &mut explicit,
        "relationship_to_protagonist",
        character.relationship_to_protagonist.as_deref(),
    );
    let mut implicit = Map::new();
    seed_insert_text(
        &mut implicit,
        "personality",
        character.personality.as_deref(),
    );
    let mut social = Map::new();
    for (target, description) in relationships {
        seed_insert_text(&mut social, target, Some(description.as_str()));
    }
    BlueprintProfileSeed {
        base_attributes: json!({
            "explicit_attributes": explicit,
            "implicit_attributes": implicit,
            "social_attributes": social,
        }),
        character_arc: blueprint_character_arc(character),
    }
}

pub(super) fn blueprint_relationship_seed_map(
    characters: &[BlueprintCharacterRecord],
    relationships: &[BlueprintRelationshipRecord],
) -> HashMap<String, Vec<(String, String)>> {
    use crate::project::normalized_blueprint_identity;

    let character_names = characters
        .iter()
        .map(|character| {
            (
                normalized_blueprint_identity(&character.name),
                character.name.clone(),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut map: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for relationship in relationships {
        let a_key = normalized_blueprint_identity(&relationship.character_a);
        let b_key = normalized_blueprint_identity(&relationship.character_b);
        let Some(a_name) = character_names.get(&a_key) else {
            continue;
        };
        let Some(b_name) = character_names.get(&b_key) else {
            continue;
        };
        let text = blueprint_relationship_seed_text(relationship);
        map.entry(a_name.clone())
            .or_default()
            .push((b_name.clone(), text.clone()));
        map.entry(b_name.clone())
            .or_default()
            .push((a_name.clone(), text));
    }
    map
}

fn seed_insert_text(target: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    target.insert(key.to_string(), Value::String(value.to_string()));
}

fn blueprint_character_arc(character: &BlueprintCharacterRecord) -> Option<Value> {
    if character
        .arc_summary
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
        && character.arc_stages.is_none()
    {
        return None;
    }
    Some(json!({
        "summary": character.arc_summary,
        "stages": character.arc_stages,
        "source": "blueprint",
    }))
}

fn blueprint_relationship_seed_text(relationship: &BlueprintRelationshipRecord) -> String {
    let mut text = relationship.relationship_type.trim().to_string();
    if let Some(description) = relationship
        .description
        .as_deref()
        .map(str::trim)
        .filter(|description| !description.is_empty())
    {
        if text.is_empty() {
            text.push_str(description);
        } else {
            text.push('：');
            text.push_str(description);
        }
    }
    if let Some(strength) = relationship.strength {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&format!("强度 {strength}/10"));
    }
    text
}
