use super::super::types::CanvasMediaSourceContext;
use crate::canvas::{CanvasDbError, PROJECT_KIND_CANVAS};
use a4n_db_models::models::project::{
    novel_project, project_source_snapshot, source_snapshot_chapter,
};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};

#[derive(Debug, Clone, FromQueryResult)]
struct CanvasProjectIdentity {
    id: i64,
    project_kind: String,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
}

async fn canvas_media_source_context<C>(
    db: &C,
    project_id: i64,
    source_chapter_id: Option<i64>,
) -> Result<CanvasMediaSourceContext, CanvasDbError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .into_model::<CanvasProjectIdentity>()
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("项目不存在: {project_id}")))?;
    if project.project_kind != PROJECT_KIND_CANVAS {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体资产只支持独立画布项目: {project_id}"
        )));
    }
    match ProjectSourceMode::from_identity(
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
    .map_err(|error| CanvasDbError::InvalidArgument(format!("项目 {project_id}: {error}")))?
    {
        ProjectSourceMode::Freeform => {
            if let Some(chapter_id) = source_chapter_id {
                return Err(CanvasDbError::InvalidArgument(format!(
                    "独立画布媒体资产不能绑定源章节: project_id={project_id}, chapter_id={chapter_id}"
                )));
            }
            Ok(CanvasMediaSourceContext {
                root_project_id: project.id,
                source_snapshot_id: None,
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
            })
        }
        mode => Err(CanvasDbError::InvalidArgument(format!(
            "画布项目来源模式无效: {mode:?}"
        ))),
    }
}

pub(in crate::canvas::artifacts) async fn canvas_media_source_context_for_update<C>(
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
        .select_only()
        .column(novel_project::Column::Id)
        .lock_exclusive()
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if let Some(source_snapshot_id) = context.source_snapshot_id {
        project_source_snapshot::Entity::find_by_id(source_snapshot_id)
            .select_only()
            .column(project_source_snapshot::Column::Id)
            .lock_exclusive()
            .into_tuple::<i64>()
            .one(db)
            .await?;
    }
    if let Some(source_snapshot_chapter_id) = context.source_snapshot_chapter_id {
        source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
            .select_only()
            .column(source_snapshot_chapter::Column::Id)
            .lock_exclusive()
            .into_tuple::<i64>()
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
