use super::contracts::{
    BlueprintCharacterDraft, BlueprintIncrementalSyncStats, BlueprintSyncError,
};
use super::identity::{blueprint_character_matches, normalized_blueprint_identity};
use a4n_core::{json_contract::is_empty_json_value, models::project::blueprint_character};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::{HashMap, HashSet};

pub async fn sync_blueprint_characters<C>(
    db: &C,
    project_id: i64,
    characters: Vec<BlueprintCharacterDraft>,
    now: NaiveDateTime,
) -> Result<BlueprintIncrementalSyncStats, BlueprintSyncError>
where
    C: ConnectionTrait,
{
    write_blueprint_characters(db, project_id, characters, now, true).await
}

pub async fn merge_blueprint_characters<C>(
    db: &C,
    project_id: i64,
    characters: Vec<BlueprintCharacterDraft>,
    now: NaiveDateTime,
) -> Result<BlueprintIncrementalSyncStats, BlueprintSyncError>
where
    C: ConnectionTrait,
{
    write_blueprint_characters(db, project_id, characters, now, false).await
}

async fn write_blueprint_characters<C>(
    db: &C,
    project_id: i64,
    characters: Vec<BlueprintCharacterDraft>,
    now: NaiveDateTime,
    delete_missing: bool,
) -> Result<BlueprintIncrementalSyncStats, BlueprintSyncError>
where
    C: ConnectionTrait,
{
    let mut seen_keys = HashSet::new();
    let mut incoming = Vec::with_capacity(characters.len());
    for character in characters {
        let key = normalized_blueprint_identity(&character.name);
        if key.is_empty() {
            return Err(BlueprintSyncError::InvalidArgument(
                "蓝图角色名称不能为空".to_string(),
            ));
        }
        if !seen_keys.insert(key.clone()) {
            return Err(BlueprintSyncError::InvalidArgument(format!(
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
        .map_err(BlueprintSyncError::Database)?;
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
            let draft = if delete_missing {
                draft
            } else {
                supplement_character_draft(&existing, draft)
            };
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
            active
                .update(db)
                .await
                .map_err(BlueprintSyncError::Database)?;
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
            .map_err(BlueprintSyncError::Database)?;
            stats.inserted += 1;
        }
    }

    if delete_missing {
        for stale in existing_by_key.into_values().flatten() {
            blueprint_character::Entity::delete_by_id(stale.id)
                .exec(db)
                .await
                .map_err(BlueprintSyncError::Database)?;
            stats.deleted += 1;
        }
    }

    Ok(stats)
}

fn supplement_character_draft(
    existing: &blueprint_character::Model,
    incoming: BlueprintCharacterDraft,
) -> BlueprintCharacterDraft {
    BlueprintCharacterDraft {
        name: existing.name.clone(),
        character_type: prefer_existing_text(&existing.character_type, incoming.character_type),
        gender: prefer_existing_optional_text(existing.gender.as_deref(), incoming.gender),
        age: prefer_existing_optional_text(existing.age.as_deref(), incoming.age),
        appearance: prefer_existing_optional_text(
            existing.appearance.as_deref(),
            incoming.appearance,
        ),
        personality: prefer_existing_optional_text(
            existing.personality.as_deref(),
            incoming.personality,
        ),
        background: prefer_existing_optional_text(
            existing.background.as_deref(),
            incoming.background,
        ),
        motivation: prefer_existing_optional_text(
            existing.motivation.as_deref(),
            incoming.motivation,
        ),
        arc_summary: prefer_existing_optional_text(
            existing.arc_summary.as_deref(),
            incoming.arc_summary,
        ),
        relationship_to_protagonist: prefer_existing_optional_text(
            existing.relationship_to_protagonist.as_deref(),
            incoming.relationship_to_protagonist,
        ),
        arc_stages: prefer_existing_json(existing.arc_stages.as_ref(), incoming.arc_stages),
    }
}

fn prefer_existing_text(existing: &str, incoming: String) -> String {
    if existing.trim().is_empty() {
        incoming
    } else {
        existing.to_string()
    }
}

fn prefer_existing_optional_text(
    existing: Option<&str>,
    incoming: Option<String>,
) -> Option<String> {
    existing
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .or(incoming)
}

fn prefer_existing_json(
    existing: Option<&serde_json::Value>,
    incoming: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    existing
        .filter(|value| !is_empty_json_value(value))
        .cloned()
        .or(incoming)
}
