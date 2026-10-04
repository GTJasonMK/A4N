use std::collections::BTreeSet;

use a4n_api_contract::character::DiscoveredCharacterProfileSummary;
use a4n_domain::character::CharacterProfileRecord;
use serde_json::{Value, json};

use crate::character::json_field_rules::{
    attribute_counts, category_attributes, required_array, required_string,
};

pub(super) fn existing_profile_for_prompt(
    profile: &CharacterProfileRecord,
) -> Result<Value, String> {
    Ok(json!({
        "name": profile.name,
        "aliases": profile_aliases(profile),
        "gender": optional_profile_text(profile.gender.as_deref()),
        "age": optional_profile_text(profile.age.as_deref()),
        "appearance": optional_profile_text(profile.appearance.as_deref()),
        "background": optional_profile_text(profile.background.as_deref()),
        "personality": optional_profile_text(profile.personality.as_deref()),
        "base_explicit_attributes": explicit_attributes_for_identity(
            &profile.base_attributes,
            "base_attributes",
        )?,
        "current_explicit_attributes": explicit_attributes_for_identity(
            &profile.current_state,
            "current_state",
        )?,
        "last_synced_chapter": profile.last_synced_chapter,
        "attribute_counts": attribute_counts(&profile.current_state)?,
    }))
}

pub(super) fn profile_for_result(
    profile: &CharacterProfileRecord,
) -> Result<DiscoveredCharacterProfileSummary, String> {
    Ok(DiscoveredCharacterProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        gender: profile.gender.clone(),
        age: profile.age.clone(),
        base_attributes: profile.base_attributes.clone(),
        current_state: profile.current_state.clone(),
        character_arc: profile.character_arc.clone(),
        voice_fingerprint: profile.voice_fingerprint.clone(),
        appearance: profile.appearance.clone(),
        background: profile.background.clone(),
        personality: profile.personality.clone(),
        last_synced_chapter: profile.last_synced_chapter,
        attribute_counts: attribute_counts(&profile.current_state)?,
        created_at: profile.created_at.to_string(),
        updated_at: profile.updated_at.to_string(),
    })
}

pub(super) fn existing_character_keys(profiles: &[CharacterProfileRecord]) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for profile in profiles {
        keys.extend(profile_character_keys(profile));
    }
    keys.retain(|key| !key.is_empty());
    keys
}

pub fn profile_character_keys(profile: &CharacterProfileRecord) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    keys.insert(character_identity_key(&profile.name));
    for alias in profile_aliases(profile) {
        keys.insert(character_identity_key(&alias));
    }
    keys.retain(|key| !key.is_empty());
    keys
}

fn profile_aliases(profile: &CharacterProfileRecord) -> Vec<String> {
    aliases_from_attributes(&profile.base_attributes)
        .into_iter()
        .chain(aliases_from_attributes(&profile.current_state))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn aliases_from_attributes(attributes: &Value) -> Vec<String> {
    let Some(aliases) = attributes
        .get("explicit_attributes")
        .and_then(Value::as_object)
        .and_then(|explicit| explicit.get("aliases"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    aliases
        .iter()
        .filter_map(|alias| alias.as_str().map(str::trim))
        .filter(|alias| !alias.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn optional_profile_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn explicit_attributes_for_identity(attributes: &Value, path: &str) -> Result<Value, String> {
    category_attributes(attributes, "explicit")
        .map_err(|err| format!("已有角色档案 {path} 无法作为角色发现上下文：{err}"))
}

pub(super) fn discovered_character_keys(character: &Value) -> Result<Vec<String>, String> {
    let mut keys = BTreeSet::new();
    keys.insert(character_identity_key(&required_string(character, "name")?));
    for alias in required_array(character, "aliases")? {
        let alias = alias
            .as_str()
            .ok_or_else(|| "角色发现字段 aliases 必须是字符串数组".to_string())?
            .trim();
        if !alias.is_empty() {
            keys.insert(character_identity_key(alias));
        }
    }
    keys.retain(|key| !key.is_empty());
    Ok(keys.into_iter().collect())
}

pub fn character_identity_key(name: &str) -> String {
    name.trim().to_lowercase()
}
