use super::contracts::{BlueprintProfileSeed, CharacterProfileSeedError};
use super::merge::merge_missing_seed_attributes;
use crate::character::{apply_profile_current_state, profile_current_state_fields};
use a4n_db_models::models::{character::protagonist_profile, project::blueprint_character};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
    TryInsertResult,
    sea_query::{CaseStatement, Expr, OnConflict},
};

pub(super) async fn insert_profile_seed<C>(
    db: &C,
    project_id: i64,
    character: &blueprint_character::Model,
    seed: &BlueprintProfileSeed,
    now: NaiveDateTime,
) -> Result<Option<protagonist_profile::Model>, CharacterProfileSeedError>
where
    C: ConnectionTrait,
{
    let name = character.name.trim();
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
        .await?;
    match result {
        TryInsertResult::Inserted(rows) if rows > 0 => Ok(None),
        TryInsertResult::Conflicted | TryInsertResult::Inserted(_) => {
            let profile = protagonist_profile::Entity::find()
                .filter(protagonist_profile::Column::ProjectId.eq(project_id))
                .filter(protagonist_profile::Column::Name.eq(name))
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or_else(|| {
                    CharacterProfileSeedError::Internal(format!(
                        "蓝图角色档案插入发生唯一键冲突，但重读同名档案失败: project_id={project_id}, name={name}"
                    ))
                })?;
            supplement_profile_from_seed(db, profile, seed, now)
                .await
                .map(Some)
        }
        TryInsertResult::Empty => Ok(None),
    }
}

async fn supplement_profile_from_seed<C>(
    db: &C,
    profile: protagonist_profile::Model,
    seed: &BlueprintProfileSeed,
    now: NaiveDateTime,
) -> Result<protagonist_profile::Model, CharacterProfileSeedError>
where
    C: ConnectionTrait,
{
    let has_timeline = crate::character::profile::timeline_profile_ids(db, &[profile.id])
        .await?
        .contains(&profile.id);
    let Some(next) = merge_profile_seed_model(profile.clone(), seed, now, has_timeline) else {
        return Ok(profile);
    };
    let mut active: protagonist_profile::ActiveModel = profile.into();
    active.base_attributes = Set(next.base_attributes);
    active.current_state = Set(next.current_state);
    active.gender = Set(next.gender);
    active.age = Set(next.age);
    active.appearance = Set(next.appearance);
    active.background = Set(next.background);
    active.personality = Set(next.personality);
    active.character_arc = Set(next.character_arc);
    active.updated_at = Set(next.updated_at);
    Ok(active.update(db).await?)
}

pub(super) fn merge_profile_seed_model(
    profile: protagonist_profile::Model,
    seed: &BlueprintProfileSeed,
    now: NaiveDateTime,
    has_timeline_records: bool,
) -> Option<protagonist_profile::Model> {
    // 蓝图只补齐初始档案；时间线开始后，缺失字段可能代表已删除的事实。
    if !crate::character::profile::initial_profile_editable(
        profile.last_synced_chapter,
        true,
        has_timeline_records,
    ) {
        return None;
    }
    let mut next = profile.clone();
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
        return None;
    }

    let fields = profile_current_state_fields(current_state);
    next.base_attributes = base_attributes;
    next.current_state = fields.current_state;
    next.gender = fields.gender;
    next.age = fields.age;
    next.appearance = fields.appearance;
    next.background = fields.background;
    next.personality = fields.personality;
    next.character_arc = next_character_arc;
    next.updated_at = now;
    Some(next)
}

pub(super) async fn update_profile_seed_models_batch<C>(
    db: &C,
    project_id: i64,
    profiles: &[protagonist_profile::Model],
) -> Result<(), CharacterProfileSeedError>
where
    C: ConnectionTrait,
{
    if profiles.is_empty() {
        return Ok(());
    }

    let mut ids = Vec::with_capacity(profiles.len());
    let mut base_attributes_case = CaseStatement::new();
    let mut current_state_case = CaseStatement::new();
    let mut gender_case = CaseStatement::new();
    let mut age_case = CaseStatement::new();
    let mut appearance_case = CaseStatement::new();
    let mut background_case = CaseStatement::new();
    let mut personality_case = CaseStatement::new();
    let mut character_arc_case = CaseStatement::new();
    let mut updated_at_case = CaseStatement::new();
    for profile in profiles {
        ids.push(profile.id);
        base_attributes_case = base_attributes_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.base_attributes.clone()).cast_as("json"),
        );
        current_state_case = current_state_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.current_state.clone()).cast_as("json"),
        );
        gender_case = gender_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.gender.clone()),
        );
        age_case = age_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.age.clone()),
        );
        appearance_case = appearance_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.appearance.clone()),
        );
        background_case = background_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.background.clone()),
        );
        personality_case = personality_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.personality.clone()),
        );
        character_arc_case = character_arc_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.character_arc.clone()).cast_as("json"),
        );
        updated_at_case = updated_at_case.case(
            protagonist_profile::Column::Id.eq(profile.id),
            Expr::value(profile.updated_at),
        );
    }

    let updated = protagonist_profile::Entity::update_many()
        .col_expr(
            protagonist_profile::Column::BaseAttributes,
            base_attributes_case
                .finally(Expr::col(protagonist_profile::Column::BaseAttributes))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::CurrentState,
            current_state_case
                .finally(Expr::col(protagonist_profile::Column::CurrentState))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::Gender,
            gender_case
                .finally(Expr::col(protagonist_profile::Column::Gender))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::Age,
            age_case
                .finally(Expr::col(protagonist_profile::Column::Age))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::Appearance,
            appearance_case
                .finally(Expr::col(protagonist_profile::Column::Appearance))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::Background,
            background_case
                .finally(Expr::col(protagonist_profile::Column::Background))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::Personality,
            personality_case
                .finally(Expr::col(protagonist_profile::Column::Personality))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::CharacterArc,
            character_arc_case
                .finally(Expr::col(protagonist_profile::Column::CharacterArc))
                .into(),
        )
        .col_expr(
            protagonist_profile::Column::UpdatedAt,
            updated_at_case
                .finally(Expr::col(protagonist_profile::Column::UpdatedAt))
                .into(),
        )
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Id.is_in(ids.clone()))
        .exec(db)
        .await?;
    if updated.rows_affected != ids.len() as u64 {
        return Err(CharacterProfileSeedError::Internal(format!(
            "蓝图角色档案批量更新影响行数不一致: updated={}, expected={}",
            updated.rows_affected,
            ids.len()
        )));
    }
    Ok(())
}
