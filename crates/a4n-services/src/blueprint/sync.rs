use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::project::{blueprint_character, blueprint_relationship};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::Value;

mod profile_seed;

pub use profile_seed::sync_character_profiles_from_blueprint;

#[derive(Debug, Clone)]
pub struct BlueprintCharacterDraft {
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub arc_summary: Option<String>,
    pub relationship_to_protagonist: Option<String>,
    pub arc_stages: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct BlueprintRelationshipDraft {
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: Option<String>,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlueprintIncrementalSyncStats {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub deleted: usize,
}

pub async fn sync_blueprint_characters<C>(
    db: &C,
    project_id: i64,
    characters: Vec<BlueprintCharacterDraft>,
    now: chrono::NaiveDateTime,
) -> Result<BlueprintIncrementalSyncStats, AppError>
where
    C: ConnectionTrait,
{
    let mut seen_keys = HashSet::new();
    let mut incoming = Vec::with_capacity(characters.len());
    for character in characters {
        let key = normalized_blueprint_identity(&character.name);
        if key.is_empty() {
            return Err(AppError::InvalidArgument(
                "蓝图角色名称不能为空".to_string(),
            ));
        }
        if !seen_keys.insert(key.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "蓝图角色重复: {}",
                character.name
            )));
        }
        incoming.push((key, character));
    }

    let existing = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut existing_by_key: HashMap<String, Vec<blueprint_character::Model>> = HashMap::new();
    for character in existing {
        existing_by_key
            .entry(normalized_blueprint_identity(&character.name))
            .or_default()
            .push(character);
    }

    let mut stats = BlueprintIncrementalSyncStats::default();
    for (key, draft) in incoming {
        if let Some(candidates) = existing_by_key.get_mut(&key)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if blueprint_character_matches(&existing, &draft) {
                stats.unchanged += 1;
                continue;
            }

            let mut active: blueprint_character::ActiveModel = existing.into();
            active.name = Set(draft.name);
            active.character_type = Set(draft.character_type);
            active.gender = Set(draft.gender);
            active.age = Set(draft.age);
            active.appearance = Set(draft.appearance);
            active.personality = Set(draft.personality);
            active.background = Set(draft.background);
            active.motivation = Set(draft.motivation);
            active.arc_summary = Set(draft.arc_summary);
            active.relationship_to_protagonist = Set(draft.relationship_to_protagonist);
            active.arc_stages = Set(draft.arc_stages);
            active.updated_at = Set(now);
            active.update(db).await.map_err(AppError::Database)?;
            stats.updated += 1;
        } else {
            blueprint_character::ActiveModel {
                project_id: Set(project_id),
                name: Set(draft.name),
                character_type: Set(draft.character_type),
                gender: Set(draft.gender),
                age: Set(draft.age),
                appearance: Set(draft.appearance),
                personality: Set(draft.personality),
                background: Set(draft.background),
                motivation: Set(draft.motivation),
                arc_summary: Set(draft.arc_summary),
                relationship_to_protagonist: Set(draft.relationship_to_protagonist),
                arc_stages: Set(draft.arc_stages),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(AppError::Database)?;
            stats.inserted += 1;
        }
    }

    for stale in existing_by_key.into_values().flatten() {
        blueprint_character::Entity::delete_by_id(stale.id)
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        stats.deleted += 1;
    }

    Ok(stats)
}

pub async fn sync_blueprint_relationships<C>(
    db: &C,
    project_id: i64,
    relationships: Vec<BlueprintRelationshipDraft>,
    now: chrono::NaiveDateTime,
) -> Result<BlueprintIncrementalSyncStats, AppError>
where
    C: ConnectionTrait,
{
    let mut seen_keys = HashSet::new();
    let mut incoming = Vec::with_capacity(relationships.len());
    for relationship in relationships {
        let relationship_type_key = normalized_blueprint_identity(&relationship.relationship_type);
        let character_a_key = normalized_blueprint_identity(&relationship.character_a);
        let character_b_key = normalized_blueprint_identity(&relationship.character_b);
        if relationship_type_key.is_empty()
            || character_a_key.is_empty()
            || character_b_key.is_empty()
        {
            return Err(AppError::InvalidArgument(
                "蓝图关系的角色和类型不能为空".to_string(),
            ));
        }
        let key = blueprint_relationship_identity_from_keys(
            &relationship_type_key,
            &character_a_key,
            &character_b_key,
        );
        if !seen_keys.insert(key.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "蓝图关系重复: {} / {} / {}",
                relationship.relationship_type, relationship.character_a, relationship.character_b
            )));
        }
        incoming.push((key, relationship));
    }

    let existing = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut existing_by_key: HashMap<String, Vec<blueprint_relationship::Model>> = HashMap::new();
    for relationship in existing {
        existing_by_key
            .entry(blueprint_relationship_model_identity(&relationship))
            .or_default()
            .push(relationship);
    }

    let mut stats = BlueprintIncrementalSyncStats::default();
    for (key, draft) in incoming {
        if let Some(candidates) = existing_by_key.get_mut(&key)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if blueprint_relationship_matches(&existing, &draft) {
                stats.unchanged += 1;
                continue;
            }

            let mut active: blueprint_relationship::ActiveModel = existing.into();
            active.relationship_type = Set(draft.relationship_type);
            active.character_a = Set(draft.character_a);
            active.character_b = Set(draft.character_b);
            active.description = Set(draft.description);
            active.strength = Set(draft.strength);
            active.evolution = Set(draft.evolution);
            active.update(db).await.map_err(AppError::Database)?;
            stats.updated += 1;
        } else {
            blueprint_relationship::ActiveModel {
                project_id: Set(project_id),
                relationship_type: Set(draft.relationship_type),
                character_a: Set(draft.character_a),
                character_b: Set(draft.character_b),
                description: Set(draft.description),
                strength: Set(draft.strength),
                evolution: Set(draft.evolution),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(AppError::Database)?;
            stats.inserted += 1;
        }
    }

    for stale in existing_by_key.into_values().flatten() {
        blueprint_relationship::Entity::delete_by_id(stale.id)
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        stats.deleted += 1;
    }

    Ok(stats)
}

pub(super) fn normalized_blueprint_identity(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn blueprint_relationship_identity_from_keys(
    relationship_type: &str,
    character_a: &str,
    character_b: &str,
) -> String {
    format!("{relationship_type}|{character_a}|{character_b}")
}

fn blueprint_relationship_model_identity(relationship: &blueprint_relationship::Model) -> String {
    format!(
        "{}|{}|{}",
        normalized_blueprint_identity(&relationship.relationship_type),
        normalized_blueprint_identity(&relationship.character_a),
        normalized_blueprint_identity(&relationship.character_b)
    )
}

fn blueprint_character_matches(
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

fn blueprint_relationship_matches(
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
