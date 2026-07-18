use super::super::{ProjectCleanupDbError, ProjectOptimizationCleanupResult};
use crate::project::validate_positive_i64;
use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_session_message,
    optimization_turn, optimization_turn_event,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn delete_project_optimization_tree<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectOptimizationCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let session_ids = optimization_session::Entity::find()
        .filter(optimization_session::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let turn_ids = if session_ids.is_empty() {
        Vec::new()
    } else {
        optimization_turn::Entity::find()
            .filter(optimization_turn::Column::SessionId.is_in(session_ids.clone()))
            .all(db)
            .await?
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>()
    };
    let patch_set_ids = if session_ids.is_empty() {
        Vec::new()
    } else {
        optimization_patch_set::Entity::find()
            .filter(optimization_patch_set::Column::SessionId.is_in(session_ids.clone()))
            .all(db)
            .await?
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>()
    };

    let deleted_patches = if patch_set_ids.is_empty() {
        0
    } else {
        optimization_patch::Entity::delete_many()
            .filter(optimization_patch::Column::PatchSetId.is_in(patch_set_ids.clone()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_patch_sets = if patch_set_ids.is_empty() {
        0
    } else {
        optimization_patch_set::Entity::delete_many()
            .filter(optimization_patch_set::Column::Id.is_in(patch_set_ids))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_events = if turn_ids.is_empty() {
        0
    } else {
        optimization_turn_event::Entity::delete_many()
            .filter(optimization_turn_event::Column::TurnId.is_in(turn_ids.clone()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_turns = if turn_ids.is_empty() {
        0
    } else {
        optimization_turn::Entity::delete_many()
            .filter(optimization_turn::Column::Id.is_in(turn_ids))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_session_messages = if session_ids.is_empty() {
        0
    } else {
        optimization_session_message::Entity::delete_many()
            .filter(optimization_session_message::Column::SessionId.is_in(session_ids.clone()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_sessions = optimization_session::Entity::delete_many()
        .filter(optimization_session::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;

    Ok(ProjectOptimizationCleanupResult {
        deleted_sessions,
        deleted_session_messages,
        deleted_turns,
        deleted_events,
        deleted_patch_sets,
        deleted_patches,
    })
}
