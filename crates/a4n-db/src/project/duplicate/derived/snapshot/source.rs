use std::collections::HashMap;

use a4n_db_models::models::project::{
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
        let rows = $module::Entity::find()
            .filter($module::Column::SnapshotId.eq($source_snapshot_id))
            .order_by_asc($module::Column::Id)
            .all($db)
            .await
            .map_err(ProjectDuplicateError::Database)?
            .into_iter()
            .map(|row| $module::ActiveModel {
                snapshot_id: Set($new_snapshot_id),
                root_project_id: Set(row.root_project_id),
                $($field: Set(row.$field),)+
                ..Default::default()
            })
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            $module::Entity::insert_many(rows)
                .exec_without_returning($db)
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

    let chapters = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(source_snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let chapter_rows = chapters
        .iter()
        .map(|chapter| source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(new_snapshot.id),
            root_project_id: Set(chapter.root_project_id),
            source_chapter_id: Set(chapter.source_chapter_id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title.clone()),
            content: Set(chapter.content.clone()),
            summary: Set(chapter.summary.clone()),
            analysis_data_json: Set(chapter.analysis_data_json.clone()),
            content_hash: Set(chapter.content_hash.clone()),
            analysis_hash: Set(chapter.analysis_hash.clone()),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let inserted_chapters = if chapter_rows.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::insert_many(chapter_rows)
            .exec_with_returning_many(db)
            .await
            .map_err(ProjectDuplicateError::Database)?
    };
    if inserted_chapters.len() != chapters.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的源快照章节数量与源记录不一致".to_string(),
            operation: "复制派生项目源快照章节".to_string(),
        });
    }
    let mut chapter_ids = HashMap::with_capacity(chapters.len());
    for (chapter, copied) in chapters.iter().zip(inserted_chapters) {
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
