use a4n_core::error::AppError;
use a4n_core::models::character::{
    attribute_event, character_portrait, character_state_index, protagonist_behavior_record,
    protagonist_deletion_mark, protagonist_profile, snapshot_reference,
};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set,
};

use crate::character::{
    apply_profile_current_state, delete_profile_in, is_text_discovered_profile,
};
use crate::generation::delete_generation_run_targets_for;

pub(super) struct ResetCharacterProfilesResult {
    pub(super) reset_character_profiles: u64,
    pub(super) deleted_character_profiles: u64,
    pub(super) deleted_character_snapshots: u64,
    pub(super) deleted_character_timeline_rows: u64,
    pub(super) deleted_character_portraits: u64,
}

pub(super) async fn reset_character_profiles_for_project(
    tx: &DatabaseTransaction,
    project_id: i64,
    now: NaiveDateTime,
) -> Result<ResetCharacterProfilesResult, AppError> {
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(tx)
        .await
        .map_err(AppError::Database)?;
    let (text_discovered_profiles, retained_profiles): (Vec<_>, Vec<_>) =
        profiles.into_iter().partition(is_text_discovered_profile);
    let text_profile_ids = text_discovered_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let retained_profile_ids = retained_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let all_profile_ids = text_profile_ids
        .iter()
        .chain(retained_profile_ids.iter())
        .copied()
        .collect::<Vec<_>>();

    let related_counts = count_character_profile_related_rows(tx, &all_profile_ids).await?;
    delete_retained_profile_related_rows(tx, &retained_profile_ids).await?;

    for profile_id in text_profile_ids.iter().copied() {
        delete_profile_in(tx, profile_id).await?;
    }

    for profile in retained_profiles {
        let mut active: protagonist_profile::ActiveModel = profile.clone().into();
        apply_profile_current_state(&mut active, profile.base_attributes.clone());
        active.last_synced_chapter = Set(0);
        active.updated_at = Set(now);
        active.update(tx).await.map_err(AppError::Database)?;
    }

    Ok(ResetCharacterProfilesResult {
        reset_character_profiles: retained_profile_ids.len() as u64,
        deleted_character_profiles: text_profile_ids.len() as u64,
        deleted_character_snapshots: related_counts.character_snapshots,
        deleted_character_timeline_rows: related_counts.attribute_events
            + related_counts.character_states
            + related_counts.behaviors
            + related_counts.deletion_marks,
        deleted_character_portraits: related_counts.character_portraits,
    })
}

struct CharacterProfileRelatedCounts {
    attribute_events: u64,
    character_states: u64,
    behaviors: u64,
    deletion_marks: u64,
    character_snapshots: u64,
    character_portraits: u64,
}

async fn count_character_profile_related_rows(
    tx: &DatabaseTransaction,
    profile_ids: &[i64],
) -> Result<CharacterProfileRelatedCounts, AppError> {
    if profile_ids.is_empty() {
        return Ok(CharacterProfileRelatedCounts {
            attribute_events: 0,
            character_states: 0,
            behaviors: 0,
            deletion_marks: 0,
            character_snapshots: 0,
            character_portraits: 0,
        });
    }

    let attribute_events = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.is_in(profile_ids.to_vec()))
        .count(tx)
        .await
        .map_err(AppError::Database)?;
    let character_states = character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.is_in(profile_ids.to_vec()))
        .count(tx)
        .await
        .map_err(AppError::Database)?;
    let behaviors = protagonist_behavior_record::Entity::find()
        .filter(protagonist_behavior_record::Column::ProfileId.is_in(profile_ids.to_vec()))
        .count(tx)
        .await
        .map_err(AppError::Database)?;
    let deletion_marks = protagonist_deletion_mark::Entity::find()
        .filter(protagonist_deletion_mark::Column::ProfileId.is_in(profile_ids.to_vec()))
        .count(tx)
        .await
        .map_err(AppError::Database)?;
    let character_snapshots = snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.is_in(profile_ids.to_vec()))
        .count(tx)
        .await
        .map_err(AppError::Database)?;
    let character_portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.is_in(profile_ids.to_vec()))
        .count(tx)
        .await
        .map_err(AppError::Database)?;

    Ok(CharacterProfileRelatedCounts {
        attribute_events,
        character_states,
        behaviors,
        deletion_marks,
        character_snapshots,
        character_portraits,
    })
}

async fn delete_retained_profile_related_rows(
    tx: &DatabaseTransaction,
    retained_profile_ids: &[i64],
) -> Result<(), AppError> {
    if retained_profile_ids.is_empty() {
        return Ok(());
    }

    attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await
        .map_err(AppError::Database)?;
    character_state_index::Entity::delete_many()
        .filter(character_state_index::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await
        .map_err(AppError::Database)?;
    protagonist_behavior_record::Entity::delete_many()
        .filter(protagonist_behavior_record::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await
        .map_err(AppError::Database)?;
    protagonist_deletion_mark::Entity::delete_many()
        .filter(protagonist_deletion_mark::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await
        .map_err(AppError::Database)?;
    snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await
        .map_err(AppError::Database)?;
    let portrait_ids = character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::Id)
        .filter(character_portrait::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .into_tuple::<i64>()
        .all(tx)
        .await
        .map_err(AppError::Database)?;
    delete_generation_run_targets_for(tx, "character_portrait", &portrait_ids).await?;
    character_portrait::Entity::delete_many()
        .filter(character_portrait::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await
        .map_err(AppError::Database)?;

    Ok(())
}
