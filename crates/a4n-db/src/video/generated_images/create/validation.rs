use super::super::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedImageRecordInput, VideoDbError,
    ensure_image_chapter_belongs_to_project_record, normalize_optional_image_record_text,
    required_image_record_text, validate_positive_image_i32, validate_positive_image_i64,
    video_project_record,
};
use chrono::Utc;
use sea_orm::ConnectionTrait;

pub(in crate::video::generated_images) async fn validate_create_generated_image_record_input<C>(
    db: &C,
    input: CreateGeneratedImageRecordDraftInput,
) -> Result<CreateGeneratedImageRecordInput, VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = validate_positive_image_i64(input.project_id, "project_id")?;
    let chapter_id = match input.chapter_id {
        Some(chapter_id) => Some(validate_positive_image_i64(chapter_id, "chapter_id")?),
        None => None,
    };
    let image_type = required_image_record_text(&input.image_type, "image_type")?;
    let file_path = required_image_record_text(&input.file_path, "file_path")?;
    let prompt = required_image_record_text(&input.prompt, "prompt")?;
    let model = required_image_record_text(&input.model, "model")?;
    let width = validate_positive_image_i32(input.width, "width")?;
    let height = validate_positive_image_i32(input.height, "height")?;
    let file_size = match input.file_size {
        Some(value) if value < 0 => {
            return Err(VideoDbError::InvalidArgument(
                "file_size 不能小于 0".to_string(),
            ));
        }
        value => value,
    };
    let panel_id = normalize_optional_image_record_text(input.panel_id.as_deref());
    match chapter_id {
        Some(chapter_id) => {
            ensure_image_chapter_belongs_to_project_record(db, project_id, chapter_id).await?;
        }
        None => {
            video_project_record(db, project_id).await?;
        }
    }

    Ok(CreateGeneratedImageRecordInput {
        project_id,
        image_type,
        file_path,
        prompt,
        model,
        width,
        height,
        file_size,
        chapter_id,
        panel_id,
        metadata_json: input.metadata_json,
        created_at: input.created_at.unwrap_or_else(|| Utc::now().naive_utc()),
    })
}
