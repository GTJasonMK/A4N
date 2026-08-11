mod creation;
mod deletion;
mod records;
mod state;
mod types;

use a4n_core::models::character::{
    attribute_event, character_state_index, protagonist_behavior_record, protagonist_deletion_mark,
    protagonist_profile, snapshot_reference,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use serde_json::Value;

pub use creation::{
    create_profile, create_profile_record, create_profile_with_generation_run,
    create_profile_with_generation_run_in,
};
pub(crate) use deletion::delete_profile_cascade_in;
pub use deletion::delete_profile_cascade_transaction;
pub use records::{
    profile_by_id, profile_by_project_name, profiles_by_project, update_profile_current_state,
};
pub(in crate::character) use records::{require_profile, require_profile_for_update};
pub use state::{
    apply_profile_current_state, normalize_base_attributes, profile_current_state_fields,
};
pub use types::{
    CharacterProfileCreateError, CharacterProfileCreateInput, CharacterProfileCurrentStateFields,
    CharacterProfileCurrentStateUpdate, CharacterProfileDeleteError,
    CharacterProfileDeletionSummary,
};

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
    let project = crate::project::project_by_id_for_update(&tx, project_id)
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

    let profile = require_profile_for_update(&tx, profile_id).await?;
    if profile.project_id != project_id {
        return Err(crate::character::CharacterDbError::InvalidArgument(
            format!("角色档案不属于当前项目: profile_id={profile_id}, project_id={project_id}"),
        ));
    }
    if profile.last_synced_chapter > 0 || profile_has_timeline_records(&tx, profile_id).await? {
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

    let mut active: protagonist_profile::ActiveModel = profile.into();
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
    if attribute_event::Entity::find()
        .select_only()
        .column(attribute_event::Column::Id)
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some()
    {
        return Ok(true);
    }
    if protagonist_behavior_record::Entity::find()
        .select_only()
        .column(protagonist_behavior_record::Column::Id)
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some()
    {
        return Ok(true);
    }
    if protagonist_deletion_mark::Entity::find()
        .select_only()
        .column(protagonist_deletion_mark::Column::Id)
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some()
    {
        return Ok(true);
    }
    if snapshot_reference::Entity::find()
        .select_only()
        .column(snapshot_reference::Column::Id)
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some()
    {
        return Ok(true);
    }
    Ok(character_state_index::Entity::find()
        .select_only()
        .column(character_state_index::Column::Id)
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some())
}

#[cfg(test)]
mod tests;
