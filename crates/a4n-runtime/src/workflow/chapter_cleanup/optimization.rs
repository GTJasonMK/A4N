use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_session_message,
    optimization_turn, optimization_turn_event,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(super) async fn cleanup_chapter_optimization_data_in<C>(
    db: &C,
    chapter_id: i64,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let sessions = optimization_session::Entity::find()
        .filter(optimization_session::Column::ChapterId.eq(chapter_id))
        .all(db)
        .await
        .map_err(|err| format!("查询章节优化会话失败: {err}"))?;
    let session_ids = sessions
        .iter()
        .map(|session| session.id)
        .collect::<Vec<_>>();
    let turn_ids = if session_ids.is_empty() {
        Vec::new()
    } else {
        optimization_turn::Entity::find()
            .filter(optimization_turn::Column::SessionId.is_in(session_ids.clone()))
            .all(db)
            .await
            .map_err(|err| format!("查询章节优化轮次失败: {err}"))?
            .iter()
            .map(|turn| turn.id)
            .collect::<Vec<_>>()
    };
    let patch_sets = optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::ChapterId.eq(chapter_id))
        .all(db)
        .await
        .map_err(|err| format!("查询章节优化补丁集失败: {err}"))?;
    let patch_set_ids = patch_sets
        .iter()
        .map(|patch_set| patch_set.id)
        .collect::<Vec<_>>();

    if !patch_set_ids.is_empty() {
        optimization_patch::Entity::delete_many()
            .filter(optimization_patch::Column::PatchSetId.is_in(patch_set_ids.clone()))
            .exec(db)
            .await
            .map_err(|err| format!("删除章节优化补丁失败: {err}"))?;
        optimization_patch_set::Entity::delete_many()
            .filter(optimization_patch_set::Column::Id.is_in(patch_set_ids))
            .exec(db)
            .await
            .map_err(|err| format!("删除章节优化补丁集失败: {err}"))?;
    }

    if session_ids.is_empty() {
        return Ok(());
    }

    if !turn_ids.is_empty() {
        optimization_turn_event::Entity::delete_many()
            .filter(optimization_turn_event::Column::TurnId.is_in(turn_ids.clone()))
            .exec(db)
            .await
            .map_err(|err| format!("删除章节优化事件失败: {err}"))?;
        optimization_turn::Entity::delete_many()
            .filter(optimization_turn::Column::Id.is_in(turn_ids))
            .exec(db)
            .await
            .map_err(|err| format!("删除章节优化轮次失败: {err}"))?;
    }

    optimization_session_message::Entity::delete_many()
        .filter(optimization_session_message::Column::SessionId.is_in(session_ids.clone()))
        .exec(db)
        .await
        .map_err(|err| format!("删除章节优化消息失败: {err}"))?;
    optimization_session::Entity::delete_many()
        .filter(optimization_session::Column::Id.is_in(session_ids))
        .exec(db)
        .await
        .map_err(|err| format!("删除章节优化会话失败: {err}"))?;
    Ok(())
}
