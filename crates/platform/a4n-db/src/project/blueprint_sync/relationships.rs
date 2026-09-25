use super::contracts::{BlueprintRelationshipDraft, BlueprintSyncError};
use super::identity::{blueprint_relationship_matches, blueprint_relationship_model_identity};
use a4n_db_models::models::project::blueprint_relationship;
use a4n_domain::json_contract::is_empty_json_value;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::collections::HashMap;

pub(in crate::project) async fn sync_blueprint_relationships<C>(
    db: &C,
    project_id: i64,
    relationships: Vec<BlueprintRelationshipDraft>,
    now: NaiveDateTime,
) -> Result<(), BlueprintSyncError>
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
) -> Result<(), BlueprintSyncError>
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
) -> Result<(), BlueprintSyncError>
where
    C: ConnectionTrait,
{
    let keys = a4n_domain::project::blueprint_relationship_identity_keys(relationships.iter().map(
        |relationship| {
            (
                relationship.relationship_type.as_str(),
                relationship.character_a.as_str(),
                relationship.character_b.as_str(),
            )
        },
    ))
    .map_err(BlueprintSyncError::InvalidArgument)?;
    let incoming = keys.into_iter().zip(relationships);

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

    let mut updates = Vec::new();
    let mut inserts = Vec::new();
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
                continue;
            }
            updates.push((existing.id, draft));
        } else {
            inserts.push(blueprint_relationship::ActiveModel {
                project_id: Set(project_id),
                relationship_type: Set(draft.relationship_type),
                character_a: Set(draft.character_a),
                character_b: Set(draft.character_b),
                description: Set(draft.description),
                strength: Set(draft.strength),
                evolution: Set(draft.evolution),
                created_at: Set(now),
                ..Default::default()
            });
        }
    }

    if !inserts.is_empty() {
        let expected = inserts.len() as u64;
        let inserted = blueprint_relationship::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await
            .map_err(BlueprintSyncError::Database)?;
        if inserted != expected {
            return Err(BlueprintSyncError::InvalidArgument(format!(
                "蓝图关系批量插入返回行数不一致: inserted={}, expected={}",
                inserted, expected
            )));
        }
    }

    if !updates.is_empty() {
        let mut relationship_type_case = CaseStatement::new();
        let mut character_a_case = CaseStatement::new();
        let mut character_b_case = CaseStatement::new();
        let mut description_case = CaseStatement::new();
        let mut strength_case = CaseStatement::new();
        let mut evolution_case = CaseStatement::new();
        let mut ids = Vec::with_capacity(updates.len());
        for (id, draft) in updates {
            relationship_type_case = relationship_type_case.case(
                blueprint_relationship::Column::Id.eq(id),
                Expr::value(draft.relationship_type),
            );
            character_a_case = character_a_case.case(
                blueprint_relationship::Column::Id.eq(id),
                Expr::value(draft.character_a),
            );
            character_b_case = character_b_case.case(
                blueprint_relationship::Column::Id.eq(id),
                Expr::value(draft.character_b),
            );
            description_case = description_case.case(
                blueprint_relationship::Column::Id.eq(id),
                Expr::value(draft.description),
            );
            strength_case = strength_case.case(
                blueprint_relationship::Column::Id.eq(id),
                Expr::value(draft.strength),
            );
            evolution_case = evolution_case.case(
                blueprint_relationship::Column::Id.eq(id),
                Expr::value(draft.evolution).cast_as("json"),
            );
            ids.push(id);
        }
        let updated = blueprint_relationship::Entity::update_many()
            .col_expr(
                blueprint_relationship::Column::RelationshipType,
                relationship_type_case
                    .finally(Expr::col(blueprint_relationship::Column::RelationshipType))
                    .into(),
            )
            .col_expr(
                blueprint_relationship::Column::CharacterA,
                character_a_case
                    .finally(Expr::col(blueprint_relationship::Column::CharacterA))
                    .into(),
            )
            .col_expr(
                blueprint_relationship::Column::CharacterB,
                character_b_case
                    .finally(Expr::col(blueprint_relationship::Column::CharacterB))
                    .into(),
            )
            .col_expr(
                blueprint_relationship::Column::Description,
                description_case
                    .finally(Expr::col(blueprint_relationship::Column::Description))
                    .into(),
            )
            .col_expr(
                blueprint_relationship::Column::Strength,
                strength_case
                    .finally(Expr::col(blueprint_relationship::Column::Strength))
                    .into(),
            )
            .col_expr(
                blueprint_relationship::Column::Evolution,
                evolution_case
                    .finally(Expr::col(blueprint_relationship::Column::Evolution))
                    .into(),
            )
            .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
            .filter(blueprint_relationship::Column::Id.is_in(ids.clone()))
            .exec(db)
            .await
            .map_err(BlueprintSyncError::Database)?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(BlueprintSyncError::InvalidArgument(format!(
                "蓝图关系批量更新返回行数不一致: updated={}, expected={}",
                updated.rows_affected,
                ids.len()
            )));
        }
    }

    if delete_missing {
        let stale_ids = existing_by_key
            .into_values()
            .flatten()
            .map(|stale| stale.id)
            .collect::<Vec<_>>();
        if !stale_ids.is_empty() {
            blueprint_relationship::Entity::delete_many()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .filter(blueprint_relationship::Column::Id.is_in(stale_ids))
                .exec(db)
                .await
                .map_err(BlueprintSyncError::Database)?;
        }
    }

    Ok(())
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
