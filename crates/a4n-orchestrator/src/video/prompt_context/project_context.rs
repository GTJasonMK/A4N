use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

use a4n_core::error::AppError;
use a4n_core::models::character::protagonist_profile;
use a4n_core::models::project::{blueprint_character, novel_blueprint};

use super::super::OrchestratorContext;

impl OrchestratorContext {
    pub(in crate::video) async fn chapter_video_project_context(
        &self,
        project_id: i64,
        chapter_analysis: Option<&Value>,
    ) -> Result<Option<Value>, AppError> {
        let character_names = video_context_character_names(chapter_analysis);
        let filter_by_names = !character_names.is_empty();
        let mut context = Map::new();
        let read_model =
            a4n_db::video::video_project_context_read_model(self.database.connection(), project_id)
                .await
                .map_err(AppError::Database)?;

        if let Some(blueprint) = read_model.blueprint {
            context.insert("blueprint".to_string(), video_blueprint_context(&blueprint));
        }

        let mut characters = BTreeMap::<String, Value>::new();
        for character in read_model.blueprint_characters {
            if filter_by_names && !character_names.contains(character.name.trim()) {
                continue;
            }
            let entry = video_character_entry(&mut characters, &character.name);
            entry.insert(
                "blueprint_character".to_string(),
                video_blueprint_character_context(&character),
            );
        }

        for profile in read_model.protagonist_profiles {
            if filter_by_names && !profile_matches_video_names(&profile, &character_names) {
                continue;
            }
            let entry = video_character_entry(&mut characters, &profile.name);
            entry.insert(
                "character_profile".to_string(),
                video_profile_context(&profile),
            );
        }

        if !characters.is_empty() {
            context.insert(
                "character_scope".to_string(),
                if filter_by_names {
                    json!({
                        "source": "chapter_analysis_character_names",
                        "names": character_names.iter().cloned().collect::<Vec<_>>(),
                    })
                } else {
                    json!({
                        "source": "all_known_project_characters",
                        "reason": "chapter_analysis 未提供可过滤角色名"
                    })
                },
            );
            context.insert(
                "characters".to_string(),
                Value::Array(characters.into_values().collect()),
            );
        }

        if context.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Value::Object(context)))
        }
    }
}

pub(in crate::video) fn video_context_character_names(
    chapter_analysis: Option<&Value>,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(analysis) = chapter_analysis else {
        return names;
    };
    collect_video_name_values(
        analysis
            .get("metadata")
            .and_then(|metadata| metadata.get("characters")),
        &mut names,
    );
    if let Some(states) = analysis.get("character_states").and_then(Value::as_object) {
        for name in states.keys() {
            insert_video_character_name(name, &mut names);
        }
    }
    if let Some(events) = analysis.get("key_events").and_then(Value::as_array) {
        for event in events {
            collect_video_name_values(event.get("involved_characters"), &mut names);
        }
    }
    names
}

pub(in crate::video) fn video_profile_context(profile: &protagonist_profile::Model) -> Value {
    let mut object = Map::new();
    insert_text_value(&mut object, "gender", profile.gender.as_deref());
    insert_text_value(&mut object, "age", profile.age.as_deref());
    insert_text_value(&mut object, "appearance", profile.appearance.as_deref());
    insert_text_value(&mut object, "background", profile.background.as_deref());
    insert_text_value(&mut object, "personality", profile.personality.as_deref());
    if let Some(explicit) = profile
        .current_state
        .get("explicit_attributes")
        .filter(|value| value.as_object().is_some_and(|obj| !obj.is_empty()))
    {
        object.insert("current_explicit_attributes".to_string(), explicit.clone());
    }
    Value::Object(object)
}

fn collect_video_name_values(value: Option<&Value>, names: &mut BTreeSet<String>) {
    match value {
        Some(Value::String(text)) => insert_video_character_name(text, names),
        Some(Value::Array(items)) => {
            for item in items {
                collect_video_name_values(Some(item), names);
            }
        }
        Some(Value::Object(obj)) => {
            collect_video_name_values(obj.get("name"), names);
            collect_video_name_values(obj.get("aliases"), names);
        }
        _ => {}
    }
}

fn insert_video_character_name(name: &str, names: &mut BTreeSet<String>) {
    let name = name.trim();
    if !name.is_empty() {
        names.insert(name.to_string());
    }
}

fn video_character_entry<'a>(
    characters: &'a mut BTreeMap<String, Value>,
    name: &str,
) -> &'a mut Map<String, Value> {
    let key = name.trim().to_string();
    let value = characters
        .entry(key.clone())
        .or_insert_with(|| json!({ "name": key }));
    value
        .as_object_mut()
        .expect("video character entry should be object")
}

fn profile_matches_video_names(
    profile: &protagonist_profile::Model,
    names: &BTreeSet<String>,
) -> bool {
    names.contains(profile.name.trim())
        || profile
            .base_attributes
            .get("explicit_attributes")
            .and_then(|attributes| attributes.get("aliases"))
            .is_some_and(|aliases| video_aliases_match(aliases, names))
        || profile
            .current_state
            .get("explicit_attributes")
            .and_then(|attributes| attributes.get("aliases"))
            .is_some_and(|aliases| video_aliases_match(aliases, names))
}

fn video_aliases_match(value: &Value, names: &BTreeSet<String>) -> bool {
    match value {
        Value::String(alias) => names.contains(alias.trim()),
        Value::Array(items) => items.iter().any(|item| video_aliases_match(item, names)),
        _ => false,
    }
}

fn video_blueprint_context(blueprint: &novel_blueprint::Model) -> Value {
    let mut object = Map::new();
    insert_text_value(&mut object, "title", Some(blueprint.title.as_str()));
    insert_text_value(&mut object, "genre", Some(blueprint.genre.as_str()));
    insert_text_value(&mut object, "style", blueprint.style.as_deref());
    insert_text_value(&mut object, "tone", blueprint.tone.as_deref());
    insert_text_value(
        &mut object,
        "one_sentence_summary",
        blueprint.one_sentence_summary.as_deref(),
    );
    if let Some(world_setting) = blueprint.world_setting.as_ref() {
        object.insert("world_setting".to_string(), world_setting.clone());
    }
    Value::Object(object)
}

fn video_blueprint_character_context(character: &blueprint_character::Model) -> Value {
    let mut object = Map::new();
    insert_text_value(
        &mut object,
        "character_type",
        Some(character.character_type.as_str()),
    );
    insert_text_value(&mut object, "gender", character.gender.as_deref());
    insert_text_value(&mut object, "age", character.age.as_deref());
    insert_text_value(&mut object, "appearance", character.appearance.as_deref());
    insert_text_value(&mut object, "personality", character.personality.as_deref());
    insert_text_value(&mut object, "background", character.background.as_deref());
    insert_text_value(&mut object, "motivation", character.motivation.as_deref());
    insert_text_value(
        &mut object,
        "relationship_to_protagonist",
        character.relationship_to_protagonist.as_deref(),
    );
    Value::Object(object)
}

fn insert_text_value(object: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        object.insert(key.to_string(), Value::String(value.to_string()));
    }
}
