use crate::source_sync::{CONFLICT_STATUS_PENDING, CONFLICT_STATUS_RESOLVED, SourceSyncDbError};
use a4n_core::models::source_sync::source_sync_conflict;
use a4n_domain::source_sync::SourceSyncResolutionStrategy;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use std::collections::BTreeSet;

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
    let old_line_ids = old_line_ids.iter().copied().collect::<BTreeSet<_>>();
    let now = Utc::now().naive_utc();
    let mut resolved = 0u64;
    for conflict in related {
        if !is_same_galgame_scene_conflict_record(&conflict, scene_id, &old_line_ids) {
            continue;
        }
        let mut active: source_sync_conflict::ActiveModel = conflict.into();
        active.status = Set(CONFLICT_STATUS_RESOLVED.to_string());
        active.resolution_strategy = Set(Some(
            SourceSyncResolutionStrategy::RegenerateScene
                .as_str()
                .to_string(),
        ));
        active.decision_json = Set(Some(json!({
            "system_action": "resolved_by_regenerate_scene",
            "galgame_scene_id": scene_id,
        })));
        active.resolved_at = Set(Some(now));
        active.update(db).await?;
        resolved += 1;
    }
    Ok(resolved)
}

pub(super) fn is_same_galgame_scene_conflict_record(
    conflict: &source_sync_conflict::Model,
    scene_id: i64,
    old_line_ids: &BTreeSet<i64>,
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
