use crate::video::VideoDbError;
use a4n_db_models::models::ai::generated_image;
use a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde_json::Value;

pub async fn video_tail_frame_images(
    db: &impl crate::DbExecutor,
) -> Result<Vec<(i64, String, Option<Value>)>, VideoDbError> {
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ImageType.eq(IMAGE_TYPE_VIDEO_TAIL_FRAME))
        .into_tuple::<(i64, String, Option<Value>)>()
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}
