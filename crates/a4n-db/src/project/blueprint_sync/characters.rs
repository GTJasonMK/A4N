use super::contracts::{BlueprintCharacterDraft, BlueprintSyncError};
use super::identity::{blueprint_character_matches, normalized_blueprint_identity};
use a4n_core::{json_contract::is_empty_json_value, models::project::blueprint_character};
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::collections::{HashMap, HashSet};

pub(in crate::project) async fn sync_blueprint_characters<C>(
    db: &C,
    project_id: i64,
    characters: Vec<BlueprintCharacterDraft>,
    now: NaiveDateTime,
) -> Result<(), BlueprintSyncError>
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
) -> Result<(), BlueprintSyncError>
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
) -> Result<(), BlueprintSyncError>
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
                supplement_character_draft(&existing, draft)
            };
            if blueprint_character_matches(&existing, &draft) {
                continue;
            }
            updates.push((existing.id, draft));
        } else {
            inserts.push(blueprint_character::ActiveModel {
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
            });
        }
    }

    if !inserts.is_empty() {
        let expected = inserts.len() as u64;
        let inserted = blueprint_character::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await
            .map_err(BlueprintSyncError::Database)?;
        if inserted != expected {
            return Err(BlueprintSyncError::InvalidArgument(format!(
                "蓝图角色批量插入返回行数不一致: inserted={}, expected={}",
                inserted, expected
            )));
        }
    }

    if !updates.is_empty() {
        let mut name_case = CaseStatement::new();
        let mut character_type_case = CaseStatement::new();
        let mut gender_case = CaseStatement::new();
        let mut age_case = CaseStatement::new();
        let mut appearance_case = CaseStatement::new();
        let mut personality_case = CaseStatement::new();
        let mut background_case = CaseStatement::new();
        let mut motivation_case = CaseStatement::new();
        let mut arc_summary_case = CaseStatement::new();
        let mut relationship_case = CaseStatement::new();
        let mut arc_stages_case = CaseStatement::new();
        let mut ids = Vec::with_capacity(updates.len());
        for (id, draft) in updates {
            name_case = name_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.name),
            );
            character_type_case = character_type_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.character_type),
            );
            gender_case = gender_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.gender),
            );
            age_case = age_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.age),
            );
            appearance_case = appearance_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.appearance),
            );
            personality_case = personality_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.personality),
            );
            background_case = background_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.background),
            );
            motivation_case = motivation_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.motivation),
            );
            arc_summary_case = arc_summary_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.arc_summary),
            );
            relationship_case = relationship_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.relationship_to_protagonist),
            );
            arc_stages_case = arc_stages_case.case(
                blueprint_character::Column::Id.eq(id),
                Expr::value(draft.arc_stages).cast_as("json"),
            );
            ids.push(id);
        }
        let updated = blueprint_character::Entity::update_many()
            .col_expr(
                blueprint_character::Column::Name,
                name_case
                    .finally(Expr::col(blueprint_character::Column::Name))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::CharacterType,
                character_type_case
                    .finally(Expr::col(blueprint_character::Column::CharacterType))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::Gender,
                gender_case
                    .finally(Expr::col(blueprint_character::Column::Gender))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::Age,
                age_case
                    .finally(Expr::col(blueprint_character::Column::Age))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::Appearance,
                appearance_case
                    .finally(Expr::col(blueprint_character::Column::Appearance))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::Personality,
                personality_case
                    .finally(Expr::col(blueprint_character::Column::Personality))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::Background,
                background_case
                    .finally(Expr::col(blueprint_character::Column::Background))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::Motivation,
                motivation_case
                    .finally(Expr::col(blueprint_character::Column::Motivation))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::ArcSummary,
                arc_summary_case
                    .finally(Expr::col(blueprint_character::Column::ArcSummary))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::RelationshipToProtagonist,
                relationship_case
                    .finally(Expr::col(
                        blueprint_character::Column::RelationshipToProtagonist,
                    ))
                    .into(),
            )
            .col_expr(
                blueprint_character::Column::ArcStages,
                arc_stages_case
                    .finally(Expr::col(blueprint_character::Column::ArcStages))
                    .into(),
            )
            .col_expr(blueprint_character::Column::UpdatedAt, Expr::value(now))
            .filter(blueprint_character::Column::ProjectId.eq(project_id))
            .filter(blueprint_character::Column::Id.is_in(ids.clone()))
            .exec(db)
            .await
            .map_err(BlueprintSyncError::Database)?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(BlueprintSyncError::InvalidArgument(format!(
                "蓝图角色批量更新返回行数不一致: updated={}, expected={}",
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
            blueprint_character::Entity::delete_many()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .filter(blueprint_character::Column::Id.is_in(stale_ids))
                .exec(db)
                .await
                .map_err(BlueprintSyncError::Database)?;
        }
    }

    Ok(())
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
