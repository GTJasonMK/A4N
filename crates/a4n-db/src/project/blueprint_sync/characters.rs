use super::contracts::{
    BlueprintCharacterDraft, BlueprintIncrementalSyncStats, BlueprintSyncError,
};
use super::identity::{blueprint_character_matches, normalized_blueprint_identity};
use a4n_core::models::project::blueprint_character;
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

    for stale in existing_by_key.into_values().flatten() {
        blueprint_character::Entity::delete_by_id(stale.id)
            .exec(db)
            .await
            .map_err(BlueprintSyncError::Database)?;
        stats.deleted += 1;
    }

    Ok(stats)
}
