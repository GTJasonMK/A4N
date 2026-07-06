use std::collections::HashMap;

use crate::character::apply_profile_current_state;
use a4n_core::error::AppError;
use a4n_core::models::{
    character::protagonist_profile,
    project::{blueprint_character, blueprint_relationship},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TryInsertResult, sea_query::OnConflict,
};
use serde_json::{Map, Value, json};

use super::normalized_blueprint_identity;

pub async fn sync_character_profiles_from_blueprint<C>(
    db: &C,
    project_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    sync_character_profiles_from_blueprint_rows(db, project_id, &characters, &relationships, now)
        .await
}

async fn sync_character_profiles_from_blueprint_rows<C>(
    db: &C,
    project_id: i64,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let relationship_map = blueprint_relationship_seed_map(characters, relationships);

    for character in characters {
        if character.name.trim().is_empty() {
            continue;
        }

        let seed = profile_seed_from_blueprint_character(
            character,
            relationship_map
                .get(&character.name)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        insert_or_update_profile_seed(db, project_id, character, &seed, now).await?;
    }

    Ok(())
}

async fn apply_profile_seed_to_existing<C>(
    db: &C,
    profile: protagonist_profile::Model,
    seed: &BlueprintProfileSeed,
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if profile.last_synced_chapter > 0 {
        return supplement_synced_profile_from_seed(db, profile, seed, now).await;
    }
    if profile.base_attributes == seed.base_attributes
        && profile.current_state == seed.base_attributes
        && profile.character_arc == seed.character_arc
    {
        return Ok(());
    }

    let mut active: protagonist_profile::ActiveModel = profile.into();
    active.base_attributes = Set(seed.base_attributes.clone());
    apply_profile_current_state(&mut active, seed.base_attributes.clone());
    active.character_arc = Set(seed.character_arc.clone());
    active.updated_at = Set(now);
    active.update(db).await.map_err(AppError::Database)?;
    Ok(())
}

async fn supplement_synced_profile_from_seed<C>(
    db: &C,
    profile: protagonist_profile::Model,
    seed: &BlueprintProfileSeed,
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut base_attributes = profile.base_attributes.clone();
    let mut current_state = profile.current_state.clone();
    let mut changed = merge_missing_seed_attributes(&mut base_attributes, &seed.base_attributes);
    changed |= merge_missing_seed_attributes(&mut current_state, &seed.base_attributes);
    let next_character_arc = match (&profile.character_arc, &seed.character_arc) {
        (None, Some(seed_arc)) => {
            changed = true;
            Some(seed_arc.clone())
        }
        _ => profile.character_arc.clone(),
    };

    if !changed {
        return Ok(());
    }

    let mut active: protagonist_profile::ActiveModel = profile.into();
    active.base_attributes = Set(base_attributes);
    apply_profile_current_state(&mut active, current_state);
    active.character_arc = Set(next_character_arc);
    active.updated_at = Set(now);
    active.update(db).await.map_err(AppError::Database)?;
    Ok(())
}

async fn insert_or_update_profile_seed<C>(
    db: &C,
    project_id: i64,
    character: &blueprint_character::Model,
    seed: &BlueprintProfileSeed,
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let name = character.name.trim();
    if let Some(profile) = find_profile_by_project_name(db, project_id, name).await? {
        return apply_profile_seed_to_existing(db, profile, seed, now).await;
    }

    let mut active = protagonist_profile::ActiveModel {
        project_id: Set(project_id),
        name: Set(name.to_string()),
        gender: Set(None),
        age: Set(None),
        base_attributes: Set(seed.base_attributes.clone()),
        current_state: Set(seed.base_attributes.clone()),
        character_arc: Set(seed.character_arc.clone()),
        voice_fingerprint: Set(None),
        appearance: Set(None),
        background: Set(None),
        personality: Set(None),
        last_synced_chapter: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    apply_profile_current_state(&mut active, seed.base_attributes.clone());
    let result = protagonist_profile::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([
                protagonist_profile::Column::ProjectId,
                protagonist_profile::Column::Name,
            ])
            .do_nothing()
            .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await
        .map_err(AppError::Database)?;

    match result {
        TryInsertResult::Inserted(rows) if rows > 0 => Ok(()),
        TryInsertResult::Conflicted | TryInsertResult::Inserted(_) => {
            let profile =
                find_profile_by_project_name(db, project_id, name)
                    .await?
                    .ok_or_else(|| {
                        AppError::Internal(anyhow::anyhow!(
                            "蓝图角色档案插入发生唯一键冲突，但重读同名档案失败: project_id={}, name={}",
                            project_id,
                            name
                        ))
                    })?;
            apply_profile_seed_to_existing(db, profile, seed, now).await
        }
        TryInsertResult::Empty => Ok(()),
    }
}

fn merge_missing_seed_attributes(target: &mut Value, seed: &Value) -> bool {
    let Some(target_root) = target.as_object_mut() else {
        return false;
    };
    let Some(seed_root) = seed.as_object() else {
        return false;
    };

    let mut changed = false;
    for category in [
        "explicit_attributes",
        "implicit_attributes",
        "social_attributes",
    ] {
        let Some(seed_attributes) = seed_root.get(category).and_then(Value::as_object) else {
            continue;
        };
        let target_attributes = target_root
            .entry(category.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !target_attributes.is_object() {
            continue;
        }
        let Some(target_attributes) = target_attributes.as_object_mut() else {
            continue;
        };
        for (key, value) in seed_attributes {
            if value_is_empty(value)
                || !value_is_empty(target_attributes.get(key).unwrap_or(&Value::Null))
            {
                continue;
            }
            target_attributes.insert(key.clone(), value.clone());
            changed = true;
        }
    }
    changed
}

fn value_is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(items) => items.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    }
}

async fn find_profile_by_project_name<C>(
    db: &C,
    project_id: i64,
    name: &str,
) -> Result<Option<protagonist_profile::Model>, AppError>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name))
        .one(db)
        .await
        .map_err(AppError::Database)
}

struct BlueprintProfileSeed {
    base_attributes: Value,
    character_arc: Option<Value>,
}

fn profile_seed_from_blueprint_character(
    character: &blueprint_character::Model,
    relationships: &[(String, String)],
) -> BlueprintProfileSeed {
    let mut explicit = Map::new();
    insert_text(
        &mut explicit,
        "character_type",
        Some(character.character_type.as_str()),
    );
    insert_text(&mut explicit, "gender", character.gender.as_deref());
    insert_text(&mut explicit, "age", character.age.as_deref());
    insert_text(&mut explicit, "appearance", character.appearance.as_deref());
    insert_text(&mut explicit, "background", character.background.as_deref());
    insert_text(&mut explicit, "motivation", character.motivation.as_deref());
    insert_text(
        &mut explicit,
        "relationship_to_protagonist",
        character.relationship_to_protagonist.as_deref(),
    );

    let mut implicit = Map::new();
    insert_text(
        &mut implicit,
        "personality",
        character.personality.as_deref(),
    );

    let mut social = Map::new();
    for (target, description) in relationships {
        insert_text(&mut social, target, Some(description.as_str()));
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

fn insert_text(target: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    target.insert(key.to_string(), Value::String(value.to_string()));
}

fn blueprint_character_arc(character: &blueprint_character::Model) -> Option<Value> {
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

fn blueprint_relationship_seed_map(
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
) -> HashMap<String, Vec<(String, String)>> {
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

fn blueprint_relationship_seed_text(relationship: &blueprint_relationship::Model) -> String {
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
