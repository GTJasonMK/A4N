use a4n_core::models::ai::generated_video;
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use sea_orm::{ConnectionTrait, EntityTrait};

use crate::canvas::CanvasDbError;

pub(super) fn canvas_video_media_origin(video: &generated_video::Model) -> &str {
    if video.video_type == generated_video::VIDEO_TYPE_CANVAS_ASSEMBLY {
        "assembled_preview"
    } else if video.model == EXTERNAL_IMPORT_MODEL {
        "imported"
    } else {
        "generated"
    }
}

pub async fn canvas_assembly_source_video<C>(
    db: &C,
    project_id: i64,
    video_id: i64,
) -> Result<generated_video::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let video = generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("成片源视频不存在: {video_id}")))?;
    if video.project_id != project_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "成片源视频不属于当前画布项目: {video_id}"
        )));
    }
    Ok(video)
}

pub async fn canvas_assembly_result_video<C>(
    db: &C,
    video_id: i64,
) -> Result<generated_video::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::InvalidArgument(format!("画布成片视频不存在: {video_id}")))
}
