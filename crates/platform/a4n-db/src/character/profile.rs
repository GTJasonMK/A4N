mod creation;
mod deletion;
mod records;
mod state;
mod types;

use a4n_db_models::models::character::{
    attribute_event, character_state_index, protagonist_behavior_record, protagonist_deletion_mark,
    protagonist_profile, snapshot_reference,
};
use a4n_db_models::models::project::novel_project;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;
use std::collections::HashSet;

pub use creation::{
    create_profile, create_profile_with_generation_run, create_profile_with_generation_run_in,
};
pub use deletion::delete_profile_cascade_transaction;
pub(crate) use deletion::delete_profiles_cascade_in;
pub(crate) use records::referenced_profile_ids;
pub use records::{
    profile_by_id, profile_by_project_name, profile_record_by_id, profile_records_by_project,
    profiles_by_project,
};
pub(in crate::character) use records::{
    require_profile, require_profile_for_update, update_profile_current_state,
};
pub(crate) use state::apply_profile_current_state;
use state::normalize_base_attributes;
pub use state::profile_current_state_fields;
pub(in crate::character) use types::CharacterProfileCurrentStateUpdate;
pub use types::{
    CharacterProfileCreateError, CharacterProfileCurrentStateFields, CharacterProfileDeleteError,
    CharacterProfileDeletionSummary,
};

#[derive(Debug, FromQueryResult)]
struct InitialProfileEditContext {
    id: i64,
    project_id: i64,
    current_state: Value,
    last_synced_chapter: i32,
}

pub(in crate::character) fn initial_profile_editable(
    last_synced_chapter: i32,
    project_active: bool,
    has_timeline_records: bool,
) -> bool {
    project_active && last_synced_chapter <= 0 && !has_timeline_records
}

pub async fn profiles_with_timeline_state<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<(protagonist_profile::Model, bool, bool)>, crate::character::CharacterDbError>
where
    C: ConnectionTrait,
{
    let (project_result, profiles_result) = futures::join!(
        novel_project::Entity::find_by_id(project_id)
            .select_only()
            .column(novel_project::Column::LifecycleGate)
            .into_tuple::<String>()
            .one(db),
        profiles_by_project(db, project_id),
    );
    let project = project_result?.ok_or_else(|| {
        crate::character::CharacterDbError::NotFound(format!("项目不存在: {project_id}"))
    })?;
    let profiles = profiles_result?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let timeline_profile_ids = timeline_profile_ids(db, &profile_ids).await?;
    let project_active = project == crate::generation::PROJECT_GATE_ACTIVE;
    Ok(profiles
        .into_iter()
        .map(|profile| {
            let timeline_started =
                profile.last_synced_chapter > 0 || timeline_profile_ids.contains(&profile.id);
            let editable = initial_profile_editable(
                profile.last_synced_chapter,
                project_active,
                timeline_profile_ids.contains(&profile.id),
            );
            (profile, timeline_started, editable)
        })
        .collect())
}

/// 直接修改尚未进入章节时间线的正式初始档案。
pub async fn update_initial_profile<C>(
    db: &C,
    project_id: i64,
    profile_id: i64,
    expected_current_state: Value,
    base_attributes: Value,
) -> Result<protagonist_profile::Model, crate::character::CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let expected_current_state = normalize_base_attributes(expected_current_state)?;
    let base_attributes = normalize_base_attributes(base_attributes)?;
    let tx = db.begin().await?;
    let project = crate::project::project_write_identity_for_update(&tx, project_id)
        .await?
        .ok_or_else(|| {
            crate::character::CharacterDbError::NotFound(format!("项目不存在: {project_id}"))
        })?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(crate::character::CharacterDbError::InvalidState {
            current: project.lifecycle_gate,
            operation: "修改初始角色档案".to_string(),
        });
    }

    let mut profile_query = protagonist_profile::Entity::find_by_id(profile_id)
        .select_only()
        .column(protagonist_profile::Column::Id)
        .column(protagonist_profile::Column::ProjectId)
        .column(protagonist_profile::Column::CurrentState)
        .column(protagonist_profile::Column::LastSyncedChapter);
    if tx.get_database_backend() != DbBackend::Sqlite {
        profile_query = profile_query.lock_exclusive();
    }
    let profile = profile_query
        .into_model::<InitialProfileEditContext>()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            crate::character::CharacterDbError::NotFound(format!("角色档案不存在: {profile_id}"))
        })?;
    if profile.project_id != project_id {
        return Err(crate::character::CharacterDbError::InvalidArgument(
            format!("角色档案不属于当前项目: profile_id={profile_id}, project_id={project_id}"),
        ));
    }
    if !initial_profile_editable(
        profile.last_synced_chapter,
        true,
        profile_has_timeline_records(&tx, profile_id).await?,
    ) {
        return Err(crate::character::CharacterDbError::InvalidState {
            current: "character_profile_timeline_started".to_string(),
            operation: "直接修改初始角色档案；请在角色详情中记录章节后的变化".to_string(),
        });
    }
    if profile.current_state != expected_current_state {
        return Err(crate::character::CharacterDbError::InvalidState {
            current: "character_profile_source_changed".to_string(),
            operation: "保存旧角色表单；角色资料已经变化，请刷新后重新编辑".to_string(),
        });
    }

    crate::generation::delete_generation_run_targets_for(&tx, "protagonist_profile", &[profile_id])
        .await?;

    let mut active = protagonist_profile::ActiveModel {
        id: Set(profile.id),
        ..Default::default()
    };
    active.base_attributes = Set(base_attributes.clone());
    apply_profile_current_state(&mut active, base_attributes);
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(updated)
}

async fn profile_has_timeline_records<C>(
    db: &C,
    profile_id: i64,
) -> Result<bool, crate::character::CharacterDbError>
where
    C: ConnectionTrait,
{
    Ok(!timeline_profile_ids(db, &[profile_id]).await?.is_empty())
}

pub(in crate::character) async fn timeline_profile_ids<C>(
    db: &C,
    profile_ids: &[i64],
) -> Result<HashSet<i64>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    if profile_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let mut timeline_profile_ids = HashSet::new();
    timeline_profile_ids.extend(
        attribute_event::Entity::find()
            .select_only()
            .column(attribute_event::Column::ProfileId)
            .filter(attribute_event::Column::ProfileId.is_in(profile_ids.iter().copied()))
            .distinct()
            .into_tuple::<i64>()
            .all(db)
            .await?,
    );
    timeline_profile_ids.extend(
        protagonist_behavior_record::Entity::find()
            .select_only()
            .column(protagonist_behavior_record::Column::ProfileId)
            .filter(
                protagonist_behavior_record::Column::ProfileId.is_in(profile_ids.iter().copied()),
            )
            .distinct()
            .into_tuple::<i64>()
            .all(db)
            .await?,
    );
    timeline_profile_ids.extend(
        protagonist_deletion_mark::Entity::find()
            .select_only()
            .column(protagonist_deletion_mark::Column::ProfileId)
            .filter(protagonist_deletion_mark::Column::ProfileId.is_in(profile_ids.iter().copied()))
            .distinct()
            .into_tuple::<i64>()
            .all(db)
            .await?,
    );
    timeline_profile_ids.extend(
        snapshot_reference::Entity::find()
            .select_only()
            .column(snapshot_reference::Column::ProfileId)
            .filter(snapshot_reference::Column::ProfileId.is_in(profile_ids.iter().copied()))
            .distinct()
            .into_tuple::<i64>()
            .all(db)
            .await?,
    );
    timeline_profile_ids.extend(
        character_state_index::Entity::find()
            .select_only()
            .column(character_state_index::Column::ProfileId)
            .filter(character_state_index::Column::ProfileId.is_in(profile_ids.iter().copied()))
            .distinct()
            .into_tuple::<i64>()
            .all(db)
            .await?,
    );
    Ok(timeline_profile_ids)
}

#[cfg(test)]
mod tests;
