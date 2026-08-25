use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::FreeformCopyMaps;
use crate::project::duplicate::freeform::remap::remap_freeform_canvas_cache_key;
use crate::project::duplicate::shared::remap_derived_optional_id;
use a4n_core::models::video_project::assembly_timeline;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::collections::HashMap;

pub(super) async fn copy_freeform_assembly_timelines_record<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
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
            if timeline.source_chapter_id.is_some() || timeline.source_snapshot_chapter_id.is_some()
            {
                return Err(ProjectDuplicateError::InvalidState {
                    current: format!("成片时间线 {} 包含派生项目上下文", timeline.id),
                    operation: "复制自由视频项目成片时间线".to_string(),
                });
            }
            Ok::<_, ProjectDuplicateError>(assembly_timeline::ActiveModel {
                project_id: Set(new_project_id),
                source_chapter_id: Set(None),
                source_snapshot_chapter_id: Set(None),
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
            operation: "复制自由视频项目成片时间线".to_string(),
        });
    }
    let mut timeline_id_map = HashMap::with_capacity(timelines.len());
    for (timeline, copied) in timelines.iter().zip(inserted) {
        timeline_id_map.insert(timeline.id, copied.id);
    }
    Ok(timeline_id_map)
}

pub(in crate::project::duplicate::freeform) async fn update_freeform_assembly_timeline_cache_keys_record<
    C,
>(
    db: &C,
    timeline_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if timeline_id_map.is_empty() {
        return Ok(());
    }
    let timelines_by_id = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::Id.is_in(timeline_id_map.values().copied()))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?
        .into_iter()
        .map(|timeline| (timeline.id, timeline))
        .collect::<HashMap<_, _>>();
    let mut preview_cache_key_case = CaseStatement::new();
    let mut changed_ids = Vec::new();
    for new_timeline_id in timeline_id_map.values().copied() {
        let timeline = timelines_by_id.get(&new_timeline_id).ok_or_else(|| {
            ProjectDuplicateError::InvalidState {
                current: format!("复制后的成片时间线不存在: {new_timeline_id}"),
                operation: "更新成片时间线缓存键".to_string(),
            }
        })?;
        let preview_cache_key =
            remap_freeform_canvas_cache_key(timeline.preview_cache_key.as_deref(), maps)?;
        if preview_cache_key == timeline.preview_cache_key {
            continue;
        }
        preview_cache_key_case = preview_cache_key_case.case(
            assembly_timeline::Column::Id.eq(new_timeline_id),
            Expr::value(preview_cache_key),
        );
        changed_ids.push(new_timeline_id);
    }
    if changed_ids.is_empty() {
        return Ok(());
    }
    let updated = assembly_timeline::Entity::update_many()
        .col_expr(
            assembly_timeline::Column::PreviewCacheKey,
            preview_cache_key_case
                .finally(Expr::col(assembly_timeline::Column::PreviewCacheKey))
                .into(),
        )
        .col_expr(assembly_timeline::Column::UpdatedAt, Expr::value(now))
        .filter(assembly_timeline::Column::Id.is_in(changed_ids.clone()))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != changed_ids.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的成片时间线缓存键数量与待回填记录不一致".to_string(),
            operation: "回填自由视频项目成片时间线缓存键".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::video_project::assembly_timeline;
    use chrono::Utc;
    use sea_orm::{EntityTrait, QueryFilter};

    #[tokio::test]
    async fn copies_timelines_and_updates_cache_keys_in_batches() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let source = crate::project::create_freeform_video_project_record(
            &db,
            1,
            crate::project::CreateFreeformVideoProjectParams {
                title: "复制时间线源".to_string(),
                description: String::new(),
            },
        )
        .await
        .unwrap();
        let target = crate::project::create_freeform_video_project_record(
            &db,
            1,
            crate::project::CreateFreeformVideoProjectParams {
                title: "复制时间线目标".to_string(),
                description: String::new(),
            },
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        let source_rows = (0..20)
            .map(|index| assembly_timeline::ActiveModel {
                project_id: Set(source.id),
                source_chapter_id: Set(None),
                source_snapshot_chapter_id: Set(None),
                version: Set(1),
                status: Set("draft".to_string()),
                preview_media_asset_id: Set(None),
                preview_cache_key: Set(Some(format!(
                    "canvas_graph:{}:node:{}",
                    10_000 + index,
                    20_000 + index
                ))),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let source_rows = assembly_timeline::Entity::insert_many(source_rows)
            .exec_with_returning_many(&db)
            .await
            .unwrap();

        let timeline_id_map = copy_freeform_assembly_timelines_record(
            &db,
            source.id,
            target.id,
            &HashMap::new(),
            now,
        )
        .await
        .unwrap();
        let mut maps = FreeformCopyMaps::default();
        for index in 0..20 {
            maps.graph_ids.insert(10_000 + index, 30_000 + index);
            maps.node_ids.insert(20_000 + index, 40_000 + index);
        }
        update_freeform_assembly_timeline_cache_keys_record(&db, &timeline_id_map, &maps, now)
            .await
            .unwrap();

        let copied = assembly_timeline::Entity::find()
            .filter(assembly_timeline::Column::ProjectId.eq(target.id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(copied.len(), source_rows.len());
        assert!(copied.iter().all(|timeline| {
            timeline
                .preview_cache_key
                .as_deref()
                .is_some_and(|key| key.starts_with("canvas_graph:30"))
        }));
    }
}
