use super::contracts::{
    BlueprintIncrementalSyncStats, BlueprintRelationshipDraft, BlueprintSyncError,
};
use super::identity::{
    blueprint_relationship_identity_from_keys, blueprint_relationship_matches,
    blueprint_relationship_model_identity, normalized_blueprint_identity,
};
use a4n_core::{json_contract::is_empty_json_value, models::project::blueprint_relationship};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::{HashMap, HashSet};

pub async fn sync_blueprint_relationships<C>(
    db: &C,
    project_id: i64,
    relationships: Vec<BlueprintRelationshipDraft>,
    now: NaiveDateTime,
) -> Result<BlueprintIncrementalSyncStats, BlueprintSyncError>
where
    C: ConnectionTrait,
{
    write_blueprint_relationships(db, project_id, relationships, now, true).await
}

pub async fn merge_blueprint_relationships<C>(
    db: &C,
    project_id: i64,
    relationships: Vec<BlueprintRelationshipDraft>,
    now: NaiveDateTime,
) -> Result<BlueprintIncrementalSyncStats, BlueprintSyncError>
where
    C: ConnectionTrait,
{
    write_blueprint_relationships(db, project_id, relationships, now, false).await
}

async fn write_blueprint_relationships<C>(
    db: &C,
    project_id: i64,
    relationships: Vec<BlueprintRelationshipDraft>,
    now: NaiveDateTime,
    delete_missing: bool,
) -> Result<BlueprintIncrementalSyncStats, BlueprintSyncError>
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
            return Err(BlueprintSyncError::InvalidArgument(
                "蓝图关系的角色和类型不能为空".to_string(),
            ));
        }
        let key = blueprint_relationship_identity_from_keys(
            &relationship_type_key,
            &character_a_key,
            &character_b_key,
        );
        if !seen_keys.insert(key.clone()) {
            return Err(BlueprintSyncError::InvalidArgument(format!(
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
        .map_err(BlueprintSyncError::Database)?;
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
            let draft = if delete_missing {
                draft
            } else {
                supplement_relationship_draft(&existing, draft)
            };
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
            active
                .update(db)
                .await
                .map_err(BlueprintSyncError::Database)?;
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
            .map_err(BlueprintSyncError::Database)?;
            stats.inserted += 1;
        }
    }

    if delete_missing {
        for stale in existing_by_key.into_values().flatten() {
            blueprint_relationship::Entity::delete_by_id(stale.id)
                .exec(db)
                .await
                .map_err(BlueprintSyncError::Database)?;
            stats.deleted += 1;
        }
    }

    Ok(stats)
}

fn supplement_relationship_draft(
    existing: &blueprint_relationship::Model,
    incoming: BlueprintRelationshipDraft,
) -> BlueprintRelationshipDraft {
    BlueprintRelationshipDraft {
        relationship_type: existing.relationship_type.clone(),
        character_a: existing.character_a.clone(),
        character_b: existing.character_b.clone(),
        description: existing
            .description
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .or(incoming.description),
        strength: existing.strength.or(incoming.strength),
        evolution: existing
            .evolution
            .as_ref()
            .filter(|value| !is_empty_json_value(value))
            .cloned()
            .or(incoming.evolution),
    }
}
