use super::state::{apply_profile_current_state, validate_base_attributes};
use crate::character::CharacterDbError;
use a4n_core::models::character::{
    attribute_event, character_state_index, protagonist_behavior_record, protagonist_deletion_mark,
    protagonist_profile, snapshot_reference,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;

/// 直接修改尚未进入章节时间线的初始角色档案。
///
/// 一旦已有时间线，基础值与章节事件之间就存在先后关系，不能把项目详情页的编辑
/// 偷换成某一章的事件，也不能原地改基线后留下旧快照。因此当前入口只处理真正的
/// “已有设定、尚未开始正文同步”场景。
pub async fn update_initial_profile<C>(
    db: &C,
    project_id: i64,
    profile_id: i64,
    expected_current_state: Value,
    base_attributes: Value,
) -> Result<protagonist_profile::Model, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_base_attributes(&expected_current_state)?;
    validate_base_attributes(&base_attributes)?;
    let tx = db.begin().await?;
    let project = crate::project::project_by_id_for_update(&tx, project_id)
        .await?
        .ok_or_else(|| CharacterDbError::NotFound(format!("项目不存在: {project_id}")))?;
    if let Some(operation) =
        crate::project::blocked_project_write_operation(&project.lifecycle_gate, "修改初始角色档案")
    {
        return Err(CharacterDbError::InvalidState {
            current: project.lifecycle_gate,
            operation,
        });
    }

    let mut query = protagonist_profile::Entity::find_by_id(profile_id);
    if tx.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let profile = query
        .one(&tx)
        .await?
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色档案不存在: {profile_id}")))?;
    if profile.project_id != project_id {
        return Err(CharacterDbError::InvalidArgument(format!(
            "角色档案不属于当前项目: profile_id={profile_id}, project_id={project_id}"
        )));
    }
    ensure_initial_profile_editable(
        profile.last_synced_chapter,
        profile_has_timeline_records(&tx, profile_id).await?,
    )?;
    ensure_initial_profile_source_matches(&profile.current_state, &expected_current_state)?;

    // 用户确认修改后，角色发现结果不再拥有这份基础档案；保留 generation_run 本身，
    // 只解除其对正式角色记录的归属。
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

fn ensure_initial_profile_source_matches(
    current_state: &Value,
    expected_current_state: &Value,
) -> Result<(), CharacterDbError> {
    if current_state != expected_current_state {
        return Err(CharacterDbError::InvalidState {
            current: "character_profile_source_changed".to_string(),
            operation: "保存旧角色表单；角色资料已经变化，请刷新后重新编辑".to_string(),
        });
    }
    Ok(())
}

fn ensure_initial_profile_editable(
    last_synced_chapter: i32,
    has_timeline_records: bool,
) -> Result<(), CharacterDbError> {
    if last_synced_chapter > 0 || has_timeline_records {
        return Err(CharacterDbError::InvalidState {
            current: "character_profile_timeline_started".to_string(),
            operation: "直接修改初始角色档案；请在角色详情中记录章节后的变化".to_string(),
        });
    }
    Ok(())
}

async fn profile_has_timeline_records<C>(db: &C, profile_id: i64) -> Result<bool, CharacterDbError>
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
mod tests {
    use super::{ensure_initial_profile_editable, ensure_initial_profile_source_matches};
    use serde_json::json;

    #[test]
    fn initial_profile_edit_stops_when_a_timeline_exists() {
        assert!(ensure_initial_profile_editable(0, false).is_ok());
        assert!(ensure_initial_profile_editable(1, false).is_err());
        assert!(ensure_initial_profile_editable(0, true).is_err());
    }

    #[test]
    fn initial_profile_edit_rejects_a_stale_form() {
        let current = json!({
            "explicit_attributes": {"occupation": "记者"},
            "implicit_attributes": {},
            "social_attributes": {"沈默": "旧友"}
        });
        assert!(ensure_initial_profile_source_matches(&current, &current).is_ok());
        assert!(
            ensure_initial_profile_source_matches(
                &current,
                &json!({
                    "explicit_attributes": {"occupation": "记者"},
                    "implicit_attributes": {},
                    "social_attributes": {}
                }),
            )
            .is_err()
        );
    }
}
