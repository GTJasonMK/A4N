use a4n_db_models::models::ai::generated_video;
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QuerySelect, Select,
};
use std::collections::HashMap;

use crate::canvas::CanvasDbError;

use super::super::types::CanvasVideoMediaSource;

pub(super) fn canvas_video_media_origin(video: &CanvasVideoMediaSource) -> &str {
    if video.video_type == a4n_domain::video_workflow::VIDEO_TYPE_CANVAS_ASSEMBLY {
        "assembled_preview"
    } else if video.model == EXTERNAL_IMPORT_MODEL {
        "imported"
    } else {
        "generated"
    }
}

pub(super) async fn canvas_assembly_source_videos<C>(
    db: &C,
    project_id: i64,
    video_ids: &[i64],
    lock_for_update: bool,
) -> Result<Vec<CanvasVideoMediaSource>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if video_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut unique_ids = video_ids.to_vec();
    unique_ids.sort_unstable();
    unique_ids.dedup();
    let mut query =
        generated_video::Entity::find().filter(generated_video::Column::Id.is_in(unique_ids));
    if lock_for_update && db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let by_id = canvas_video_media_source_query(query)
        .into_model::<CanvasVideoMediaSource>()
        .all(db)
        .await?
        .into_iter()
        .map(|video| (video.id, video))
        .collect::<HashMap<_, _>>();
    video_ids
        .iter()
        .map(|video_id| {
            let video = by_id.get(video_id).cloned().ok_or_else(|| {
                CanvasDbError::InvalidArgument(format!("成片源视频不存在: {video_id}"))
            })?;
            if video.project_id != project_id {
                return Err(CanvasDbError::InvalidArgument(format!(
                    "成片源视频不属于当前画布项目: {video_id}"
                )));
            }
            Ok(video)
        })
        .collect()
}

pub(super) async fn canvas_assembly_result_video<C>(
    db: &C,
    video_id: i64,
) -> Result<CanvasVideoMediaSource, CanvasDbError>
where
    C: ConnectionTrait,
{
    canvas_video_media_source_query(generated_video::Entity::find_by_id(video_id))
        .into_model::<CanvasVideoMediaSource>()
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("画布成片视频不存在: {video_id}")))
}

pub(super) async fn canvas_assembly_result_video_for_update<C>(
    db: &C,
    project_id: i64,
    video_id: i64,
) -> Result<CanvasVideoMediaSource, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = generated_video::Entity::find_by_id(video_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let video = canvas_video_media_source_query(query)
        .into_model::<CanvasVideoMediaSource>()
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("视频不存在: {video_id}")))?;
    if video.project_id != project_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "视频不属于当前项目: {}",
            video.id
        )));
    }
    Ok(video)
}

fn canvas_video_media_source_query(
    query: Select<generated_video::Entity>,
) -> Select<generated_video::Entity> {
    query
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::ProjectId)
        .column(generated_video::Column::ChapterId)
        .column(generated_video::Column::VideoType)
        .column(generated_video::Column::WorkflowMode)
        .column(generated_video::Column::FilePath)
        .column(generated_video::Column::ThumbnailPath)
        .column(generated_video::Column::Model)
        .column(generated_video::Column::Width)
        .column(generated_video::Column::Height)
        .column(generated_video::Column::DurationSecs)
        .column(generated_video::Column::Fps)
        .column(generated_video::Column::FileSize)
        .column(generated_video::Column::Status)
        .column(generated_video::Column::ProviderJobId)
        .column(generated_video::Column::MetadataJson)
}
