use super::cleanup::delete_timeline_artifacts_from_chapter_checked;
use super::contracts::CharacterProfileRollbackResult;
use crate::character::{
    CharacterDbError, CharacterProfileCurrentStateUpdate, profile_current_state_fields,
    require_profile, rewind_deletion_marks_after, snapshot_attributes_from_data,
    snapshot_reference_by_chapter, update_profile_current_state,
};
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn rollback_profile_to_chapter<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
) -> Result<CharacterProfileRollbackResult, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if chapter_number < 0 {
        return Err(CharacterDbError::InvalidArgument(
            "回滚章节号不能小于 0".to_string(),
        ));
    }

    let tx = db.begin().await?;
    let profile = require_profile(&tx, profile_id).await?;
    let project_id = profile.project_id;
    let (restored_state, cleanup_start_chapter) = if chapter_number == 0 {
        (profile.base_attributes.clone(), 1)
    } else {
        let target_snapshot = snapshot_reference_by_chapter(&tx, profile_id, chapter_number)
            .await?
            .ok_or_else(|| CharacterDbError::InvalidState {
                current: format!("第 {chapter_number} 章没有角色快照"),
                operation: "回滚角色档案到该章节".to_string(),
            })?;
        (
            snapshot_attributes_from_data(&target_snapshot.data, chapter_number)?,
            chapter_number + 1,
        )
    };

    let deleted_timeline = delete_timeline_artifacts_from_chapter_checked(
        &tx,
        profile_id,
        cleanup_start_chapter,
        None,
    )
    .await?;
    let deletion_mark_rewind = rewind_deletion_marks_after(&tx, profile_id, chapter_number).await?;
    let restored_fields = profile_current_state_fields(restored_state);
    update_profile_current_state(
        &tx,
        profile,
        CharacterProfileCurrentStateUpdate {
            gender: restored_fields.gender,
            age: restored_fields.age,
            appearance: restored_fields.appearance,
            background: restored_fields.background,
            personality: restored_fields.personality,
            current_state: restored_fields.current_state,
            last_synced_chapter: Some(chapter_number),
        },
    )
    .await?;
    tx.commit().await?;

    Ok(CharacterProfileRollbackResult {
        project_id,
        deleted_timeline,
        deletion_mark_rewind,
        last_synced_chapter: chapter_number,
    })
}
