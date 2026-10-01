use crate::galgame::{GalgameGenerationRecordError, validate_galgame_generation_id};
use crate::video::media_mapping::generated_image_record_from_model;
use a4n_db_models::models::ai::generated_image;
use a4n_domain::media::GeneratedImageRecord;
use sea_orm::{DatabaseConnection, EntityTrait};

pub async fn galgame_generated_image_by_id(
    db: &DatabaseConnection,
    image_id: i64,
    project_id: i64,
) -> Result<GeneratedImageRecord, GalgameGenerationRecordError> {
    validate_galgame_generation_id(image_id, "image_id")?;
    validate_galgame_generation_id(project_id, "project_id")?;
    let generated = generated_image::Entity::find_by_id(image_id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!("Galgame 图片产物不存在: {image_id}"))
        })?;
    if generated.project_id != project_id {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "图片不属于当前 Galgame 项目: {}",
            generated.id
        )));
    }
    if generated.file_path.trim().is_empty() {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 图片缺少文件路径，不能登记为媒体资产: {}",
            generated.id
        )));
    }
    Ok(generated_image_record_from_model(generated))
}
