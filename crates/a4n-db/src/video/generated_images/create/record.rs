use super::super::CreateGeneratedImageRecordInput;
use a4n_db_models::models::ai::generated_image;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, Set};

pub(in crate::video::generated_images) async fn insert_generated_image_record<C>(
    db: &C,
    input: CreateGeneratedImageRecordInput,
) -> Result<generated_image::Model, DbErr>
where
    C: ConnectionTrait,
{
    generated_image::ActiveModel {
        project_id: Set(input.project_id),
        image_type: Set(input.image_type),
        file_path: Set(input.file_path),
        prompt: Set(input.prompt),
        model: Set(input.model),
        width: Set(input.width),
        height: Set(input.height),
        file_size: Set(input.file_size),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(input.chapter_id),
        panel_id: Set(input.panel_id),
        metadata_json: Set(input.metadata_json),
        created_at: Set(input.created_at),
        ..Default::default()
    }
    .insert(db)
    .await
}
