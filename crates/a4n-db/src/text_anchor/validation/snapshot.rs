use a4n_core::models::project::{project_source_snapshot, source_snapshot_chapter};
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};

use crate::text_anchor::{TextAnchorDbError, TextAnchorDbResult};

pub(super) async fn validate_snapshot_filters<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
) -> TextAnchorDbResult<()>
where
    C: ConnectionTrait,
{
    match (source_snapshot_id, source_snapshot_chapter_id) {
        (None, Some(_)) => Err(TextAnchorDbError::InvalidArgument(
            "source_snapshot_chapter_id 需要同时提供 source_snapshot_id".to_string(),
        )),
        (Some(snapshot_id), chapter_id) => {
            let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_id)
                .select_only()
                .column(project_source_snapshot::Column::RootProjectId)
                .into_tuple::<i64>()
                .one(db)
                .await?
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("源文本快照不存在: {snapshot_id}"))
                })?;
            if snapshot != root_project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "源文本快照不属于文本根项目: snapshot_id={snapshot_id}, root_project_id={root_project_id}"
                )));
            }
            if let Some(chapter_id) = chapter_id {
                let chapter = source_snapshot_chapter::Entity::find_by_id(chapter_id)
                    .select_only()
                    .column(source_snapshot_chapter::Column::SnapshotId)
                    .column(source_snapshot_chapter::Column::RootProjectId)
                    .into_tuple::<(i64, i64)>()
                    .one(db)
                    .await?
                    .ok_or_else(|| {
                        TextAnchorDbError::NotFound(format!("源快照章节不存在: {chapter_id}"))
                    })?;
                if chapter.0 != snapshot_id || chapter.1 != root_project_id {
                    return Err(TextAnchorDbError::InvalidArgument(format!(
                        "源快照章节不属于目标源快照: source_snapshot_chapter_id={chapter_id}"
                    )));
                }
            }
            Ok(())
        }
        (None, None) => Ok(()),
    }
}
