use crate::video::{VideoDbError, validate_positive_video_i64};
use a4n_core::models::{chapter::chapter, project::novel_project};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ConnectionTrait, EntityTrait, FromQueryResult, QuerySelect};

#[derive(Debug, Clone, FromQueryResult)]
pub struct VideoProjectIdentity {
    pub id: i64,
    pub project_kind: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
}

pub(crate) async fn video_project_record<C>(
    db: &C,
    project_id: i64,
) -> Result<VideoProjectIdentity, VideoDbError>
where
    C: ConnectionTrait,
{
    validate_positive_video_i64(project_id, "project_id")?;
    novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .into_model::<VideoProjectIdentity>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("项目不存在: {project_id}")))
}

pub async fn video_chapter_record<C>(
    db: &C,
    chapter_id: i64,
) -> Result<chapter::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("章节不存在: {chapter_id}")))
}

pub(crate) async fn video_chapter_project_id_record<C>(
    db: &C,
    chapter_id: i64,
) -> Result<i64, VideoDbError>
where
    C: ConnectionTrait,
{
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    chapter::Entity::find_by_id(chapter_id)
        .select_only()
        .column(chapter::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("章节不存在: {chapter_id}")))
}

pub async fn ensure_video_chapter_belongs_to_project_record<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let (project_result, chapter_project_result) = futures::join!(
        video_project_record(db, project_id),
        video_chapter_project_id_record(db, chapter_id),
    );
    let project = project_result?;
    let chapter_project_id = chapter_project_result?;
    match project.project_kind.as_str() {
        "text" if chapter_project_id == project.id => Ok(()),
        "text" => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于文本项目 {project_id}"
        ))),
        "video" if project.root_project_id == Some(chapter_project_id) => Ok(()),
        "video" => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于视频项目 {project_id} 绑定的根文本项目"
        ))),
        other => Err(VideoDbError::InvalidArgument(format!(
            "视频链路不支持项目类型 {other}: project_id={project_id}"
        ))),
    }
}

pub async fn video_asset_project_ids_record<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<i64>, VideoDbError>
where
    C: ConnectionTrait,
{
    let project = video_project_record(db, project_id).await?;
    match project.project_kind.as_str() {
        "text" => Ok(vec![project.id]),
        "video"
            if ProjectSourceMode::from_identity(
                &project.project_kind,
                project.root_project_id,
                project.source_snapshot_id,
            )
            .map_err(|error| {
                VideoDbError::InvalidArgument(format!("项目 {project_id}: {error}"))
            })? == ProjectSourceMode::Freeform =>
        {
            Ok(vec![project.id])
        }
        "video" => Ok(vec![
            project.id,
            project.root_project_id.ok_or_else(|| {
                VideoDbError::InvalidArgument(format!("视频项目缺少 root_project_id: {project_id}"))
            })?,
        ]),
        other => Err(VideoDbError::InvalidArgument(format!(
            "视频链路不支持项目类型 {other}: project_id={project_id}"
        ))),
    }
}
