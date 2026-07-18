use super::contracts::{BlueprintProfileSeed, CharacterProfileSeedError};
use super::merge::merge_missing_seed_attributes;
use crate::character::{apply_profile_current_state, profile_by_project_name};
use a4n_core::models::{character::protagonist_profile, project::blueprint_character};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, EntityTrait, Set, TryInsertResult, sea_query::OnConflict,
};

pub(super) async fn insert_or_update_profile_seed<C>(
    db: &C,
    project_id: i64,
    character: &blueprint_character::Model,
    seed: &BlueprintProfileSeed,
    now: NaiveDateTime,
) -> Result<(), CharacterProfileSeedError>
where
    C: ConnectionTrait,
{
    let name = character.name.trim();
    if let Some(profile) = profile_by_project_name(db, project_id, name).await? {
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
        .await?;
    match result {
        TryInsertResult::Inserted(rows) if rows > 0 => Ok(()),
        TryInsertResult::Conflicted | TryInsertResult::Inserted(_) => {
            let profile = profile_by_project_name(db, project_id, name)
                .await?
                .ok_or_else(|| {
                    CharacterProfileSeedError::Internal(format!(
                        "蓝图角色档案插入发生唯一键冲突，但重读同名档案失败: project_id={project_id}, name={name}"
                    ))
                })?;
            apply_profile_seed_to_existing(db, profile, seed, now).await
        }
        TryInsertResult::Empty => Ok(()),
    }
}

async fn apply_profile_seed_to_existing<C>(
    db: &C,
    profile: protagonist_profile::Model,
    seed: &BlueprintProfileSeed,
    now: NaiveDateTime,
) -> Result<(), CharacterProfileSeedError>
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
    active.update(db).await?;
    Ok(())
}

async fn supplement_synced_profile_from_seed<C>(
    db: &C,
    profile: protagonist_profile::Model,
    seed: &BlueprintProfileSeed,
    now: NaiveDateTime,
) -> Result<(), CharacterProfileSeedError>
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
    active.update(db).await?;
    Ok(())
}
