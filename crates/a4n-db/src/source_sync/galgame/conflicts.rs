use crate::source_sync::{CONFLICT_STATUS_PENDING, CONFLICT_STATUS_RESOLVED, SourceSyncDbError};
use a4n_db_models::models::source_sync::source_sync_conflict;
use a4n_domain::source_sync::SourceSyncResolutionStrategy;
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, sea_query::Expr};
use serde_json::json;
use std::collections::HashSet;

pub async fn resolve_related_galgame_scene_conflict_records<C>(
    db: &C,
    sync_run_id: i64,
    current_conflict_id: i64,
    scene_id: i64,
    old_line_ids: &[i64],
) -> Result<u64, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let related = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(sync_run_id))
        .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
        .filter(source_sync_conflict::Column::Id.ne(current_conflict_id))
        .all(db)
        .await?;
    let old_line_ids = old_line_ids.iter().copied().collect::<HashSet<_>>();
    let matching_ids = related
        .into_iter()
        .filter(|conflict| is_same_galgame_scene_conflict_record(conflict, scene_id, &old_line_ids))
        .map(|conflict| conflict.id)
        .collect::<Vec<_>>();
    if matching_ids.is_empty() {
        return Ok(0);
    }
    let now = Utc::now().naive_utc();
    source_sync_conflict::Entity::update_many()
        .col_expr(
            source_sync_conflict::Column::Status,
            Expr::value(CONFLICT_STATUS_RESOLVED),
        )
        .col_expr(
            source_sync_conflict::Column::ResolutionStrategy,
            Expr::value(Some(
                SourceSyncResolutionStrategy::RegenerateScene
                    .as_str()
                    .to_string(),
            )),
        )
        .col_expr(
            source_sync_conflict::Column::DecisionJson,
            Expr::value(Some(json!({
                "system_action": "resolved_by_regenerate_scene",
                "galgame_scene_id": scene_id,
            }))),
        )
        .col_expr(
            source_sync_conflict::Column::ResolvedAt,
            Expr::value(Some(now)),
        )
        .filter(source_sync_conflict::Column::Id.is_in(matching_ids.clone()))
        .exec(db)
        .await?;
    Ok(matching_ids.len() as u64)
}

pub(super) fn is_same_galgame_scene_conflict_record(
    conflict: &source_sync_conflict::Model,
    scene_id: i64,
    old_line_ids: &HashSet<i64>,
) -> bool {
    if conflict.conflict_scope != "text_anchor" {
        return false;
    }
    match conflict.target_kind.as_str() {
        "galgame_scene" => conflict.target_id == scene_id,
        "galgame_line" => {
            conflict
                .affected_refs_json
                .get("galgame_scene_id")
                .and_then(serde_json::Value::as_i64)
                == Some(scene_id)
                || old_line_ids.contains(&conflict.target_id)
        }
        _ => false,
    }
}
