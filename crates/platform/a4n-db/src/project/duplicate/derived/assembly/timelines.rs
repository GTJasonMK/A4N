use std::collections::HashMap;

use a4n_db_models::models::video_project::assembly_timeline;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::project::duplicate::{ProjectDuplicateError, shared::remap_derived_optional_id};

use super::super::types::DuplicatedDerivedSourceSnapshot;

pub(in crate::project::duplicate::derived) async fn copy_derived_assembly_timelines<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    duplicated_snapshot: &DuplicatedDerivedSourceSnapshot,
    asset_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let timelines = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(source_project_id))
        .order_by_asc(assembly_timeline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if timelines.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = timelines
        .iter()
        .map(|timeline| {
            Ok::<_, ProjectDuplicateError>(assembly_timeline::ActiveModel {
                project_id: Set(new_project_id),
                source_chapter_id: Set(timeline.source_chapter_id),
                source_snapshot_chapter_id: Set(remap_derived_optional_id(
                    &duplicated_snapshot.chapter_ids,
                    timeline.source_snapshot_chapter_id,
                    "assembly_timeline.source_snapshot_chapter_id",
                )?),
                version: Set(timeline.version),
                status: Set(timeline.status.clone()),
                preview_media_asset_id: Set(remap_derived_optional_id(
                    asset_id_map,
                    timeline.preview_media_asset_id,
                    "assembly_timeline.preview_media_asset_id",
                )?),
                preview_cache_key: Set(timeline.preview_cache_key.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    let inserted = assembly_timeline::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != timelines.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的成片时间线数量与源记录不一致".to_string(),
            operation: "复制派生项目成片时间线".to_string(),
        });
    }
    let mut timeline_id_map = HashMap::with_capacity(timelines.len());
    for (timeline, copied) in timelines.iter().zip(inserted) {
        timeline_id_map.insert(timeline.id, copied.id);
    }
    Ok(timeline_id_map)
}
