use std::collections::HashMap;

use a4n_core::models::project::{
    project_source_snapshot, source_snapshot_chapter, source_snapshot_outline,
    source_snapshot_profile,
};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::project::duplicate::ProjectDuplicateError;

use super::super::types::DuplicatedDerivedSourceSnapshot;

macro_rules! copy_derived_snapshot_detail_rows {
    ($db:expr, $source_snapshot_id:expr, $new_snapshot_id:expr, $module:ident, [$($field:ident),+ $(,)?]) => {{
        for row in $module::Entity::find()
            .filter($module::Column::SnapshotId.eq($source_snapshot_id))
            .order_by_asc($module::Column::Id)
            .all($db)
            .await
            .map_err(ProjectDuplicateError::Database)?
        {
            $module::ActiveModel {
                snapshot_id: Set($new_snapshot_id),
                root_project_id: Set(row.root_project_id),
                $($field: Set(row.$field),)+
                ..Default::default()
            }
            .insert($db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        }
    }};
}

pub(super) async fn duplicate_derived_source_snapshot<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    now: NaiveDateTime,
) -> Result<DuplicatedDerivedSourceSnapshot, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let snapshot = project_source_snapshot::Entity::find_by_id(source_snapshot_id)
        .one(db)
        .await
        .map_err(ProjectDuplicateError::Database)?
        .ok_or_else(|| ProjectDuplicateError::InvalidState {
            current: format!("源文本快照不存在: {source_snapshot_id}"),
            operation: "复制派生项目源快照".to_string(),
        })?;
    if snapshot.root_project_id != root_project_id {
        return Err(ProjectDuplicateError::InvalidState {
            current: format!("源文本快照 {source_snapshot_id} 不属于根文本项目 {root_project_id}"),
            operation: "复制派生项目源快照".to_string(),
        });
    }
    let new_snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(snapshot.root_project_id),
        source_version_hash: Set(snapshot.source_version_hash),
        blueprint_version_id: Set(snapshot.blueprint_version_id),
        chapter_scope_json: Set(snapshot.chapter_scope_json),
        chapter_hashes_json: Set(snapshot.chapter_hashes_json),
        profile_hashes_json: Set(snapshot.profile_hashes_json),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectDuplicateError::Database)?;

    let mut chapter_ids = HashMap::new();
    for chapter in source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(source_snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?
    {
        let copied = source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(new_snapshot.id),
            root_project_id: Set(chapter.root_project_id),
            source_chapter_id: Set(chapter.source_chapter_id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title),
            content: Set(chapter.content),
            summary: Set(chapter.summary),
            analysis_data_json: Set(chapter.analysis_data_json),
            content_hash: Set(chapter.content_hash),
            analysis_hash: Set(chapter.analysis_hash),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        chapter_ids.insert(chapter.id, copied.id);
    }
    copy_derived_snapshot_detail_rows!(
        db,
        source_snapshot_id,
        new_snapshot.id,
        source_snapshot_outline,
        [source_outline_id, outline_kind, payload_json, payload_hash]
    );
    copy_derived_snapshot_detail_rows!(
        db,
        source_snapshot_id,
        new_snapshot.id,
        source_snapshot_profile,
        [profile_kind, source_profile_id, payload_json, payload_hash]
    );

    Ok(DuplicatedDerivedSourceSnapshot {
        snapshot: new_snapshot,
        chapter_ids,
    })
}
