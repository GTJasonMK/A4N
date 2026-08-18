use super::super::types::CanvasMediaSourceContext;
use crate::canvas::{CanvasDbError, PROJECT_KIND_VIDEO};
use a4n_core::models::project::{novel_project, project_source_snapshot, source_snapshot_chapter};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub async fn canvas_media_source_context<C>(
    db: &C,
    project_id: i64,
    source_chapter_id: Option<i64>,
) -> Result<CanvasMediaSourceContext, CanvasDbError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("项目不存在: {project_id}")))?;
    if project.project_kind != PROJECT_KIND_VIDEO {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体资产只支持视频项目: {project_id}"
        )));
    }
    match project.source_mode().map_err(|error| {
        CanvasDbError::InvalidArgument(format!("项目 {project_id}: {error}"))
    })? {
        ProjectSourceMode::Freeform => {
            if let Some(chapter_id) = source_chapter_id {
                return Err(CanvasDbError::InvalidArgument(format!(
                    "freeform 视频项目媒体资产不能绑定源章节: project_id={project_id}, chapter_id={chapter_id}"
                )));
            }
            Ok(CanvasMediaSourceContext {
                root_project_id: project.id,
                source_snapshot_id: None,
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
            })
        }
        ProjectSourceMode::Derived => {
            let context = crate::project::derived_project_family_context(db, project_id)
                .await
                .map_err(project_family_error_to_canvas_error)?;
            let root_project_id = context.root_project.id;
            let source_snapshot_id = context.source_snapshot.id;
            let source_snapshot_chapter_id = if let Some(chapter_id) = source_chapter_id {
                Some(
                    source_snapshot_chapter::Entity::find()
                        .filter(source_snapshot_chapter::Column::SnapshotId.eq(source_snapshot_id))
                        .filter(source_snapshot_chapter::Column::RootProjectId.eq(root_project_id))
                        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(chapter_id))
                        .one(db)
                        .await?
                        .ok_or_else(|| {
                            CanvasDbError::InvalidArgument(format!(
                                "派生视频项目 {project_id} 的源快照不包含章节 {chapter_id}"
                            ))
                        })?
                        .id,
                )
            } else {
                None
            };
            Ok(CanvasMediaSourceContext {
                root_project_id,
                source_snapshot_id: Some(source_snapshot_id),
                source_chapter_id,
                source_snapshot_chapter_id,
            })
        }
        ProjectSourceMode::Self_ => Err(CanvasDbError::InvalidArgument(
            "画布媒体资产不支持文本项目".to_string(),
        )),
    }
}

pub(super) async fn canvas_media_source_context_for_update<C>(
    db: &C,
    project_id: i64,
    source_chapter_id: Option<i64>,
) -> Result<CanvasMediaSourceContext, CanvasDbError>
where
    C: ConnectionTrait,
{
    let context = canvas_media_source_context(db, project_id, source_chapter_id).await?;
    if db.get_database_backend() == DbBackend::Sqlite {
        return Ok(context);
    }

    let mut project_ids = vec![project_id, context.root_project_id];
    project_ids.sort_unstable();
    project_ids.dedup();
    novel_project::Entity::find()
        .filter(novel_project::Column::Id.is_in(project_ids))
        .order_by_asc(novel_project::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;
    if let Some(source_snapshot_id) = context.source_snapshot_id {
        project_source_snapshot::Entity::find_by_id(source_snapshot_id)
            .lock_exclusive()
            .one(db)
            .await?;
    }
    if let Some(source_snapshot_chapter_id) = context.source_snapshot_chapter_id {
        source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
            .lock_exclusive()
            .one(db)
            .await?;
    }

    let locked_context = canvas_media_source_context(db, project_id, source_chapter_id).await?;
    if locked_context != context {
        return Err(CanvasDbError::InvalidArgument(format!(
            "项目 {project_id} 的画布媒体源上下文在登记期间发生变化，请重试"
        )));
    }
    Ok(locked_context)
}

fn project_family_error_to_canvas_error(
    error: crate::project::ProjectFamilyDbError,
) -> CanvasDbError {
    match error {
        crate::project::ProjectFamilyDbError::InvalidArgument(message)
        | crate::project::ProjectFamilyDbError::NotFound(message) => {
            CanvasDbError::InvalidArgument(message)
        }
        crate::project::ProjectFamilyDbError::Database(error) => CanvasDbError::Database(error),
    }
}
