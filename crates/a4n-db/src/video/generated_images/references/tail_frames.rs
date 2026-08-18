use crate::video::VideoDbError;
use a4n_core::models::ai::generated_image;
use a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn video_tail_frame_images(
    db: &DatabaseConnection,
) -> Result<Vec<generated_image::Model>, VideoDbError> {
    generated_image::Entity::find()
        .filter(generated_image::Column::ImageType.eq(IMAGE_TYPE_VIDEO_TAIL_FRAME))
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}
