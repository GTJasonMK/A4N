use super::{
    super::{
        UpdateGeneratedImageRecordInput, VideoDbError, normalize_optional_video_record_text,
        required_video_record_text,
    },
    validation::{
        validate_generated_image_update_chapter_binding_fields,
        validate_generated_image_update_clear_flags, validate_generated_image_update_image_type,
        validate_generated_image_update_panel_binding_fields,
    },
};
use a4n_core::models::ai::generated_image;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, FromQueryResult, QuerySelect, Set};

#[derive(Debug, FromQueryResult)]
struct GeneratedImageUpdateContext {
    id: i64,
    project_id: i64,
    image_type: String,
    chapter_id: Option<i64>,
    panel_id: Option<String>,
}

pub async fn update_generated_image_record<C>(
    db: &C,
    image_id: i64,
    input: UpdateGeneratedImageRecordInput,
) -> Result<generated_image::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    validate_generated_image_update_clear_flags(&input)?;
    let image = generated_image::Entity::find_by_id(image_id)
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ProjectId)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::ChapterId)
        .column(generated_image::Column::PanelId)
        .into_model::<GeneratedImageUpdateContext>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("图片不存在".to_string()))?;

    let mut active = generated_image::ActiveModel {
        id: Set(image.id),
        ..Default::default()
    };
    if let Some(prompt) = input.prompt {
        active.prompt = Set(required_video_record_text(&prompt, "prompt")?);
    }
    if let Some(image_type) = input.image_type {
        let image_type = required_video_record_text(&image_type, "image_type")?;
        validate_generated_image_update_image_type(&image.image_type, &image_type)?;
        active.image_type = Set(image_type);
    }
    if input.clear_chapter_id {
        validate_generated_image_update_chapter_binding_fields(
            image.id,
            &image.image_type,
            image.chapter_id,
            None,
        )?;
        active.chapter_id = Set(None);
    } else if let Some(chapter_id) = input.chapter_id {
        let chapter_id = super::super::validate_positive_image_i64(chapter_id, "chapter_id")?;
        validate_generated_image_update_chapter_binding_fields(
            image.id,
            &image.image_type,
            image.chapter_id,
            Some(chapter_id),
        )?;
        super::super::ensure_image_chapter_belongs_to_project_record(
            db,
            image.project_id,
            chapter_id,
        )
        .await?;
        active.chapter_id = Set(Some(chapter_id));
    }
    if input.clear_panel_id {
        validate_generated_image_update_panel_binding_fields(
            image.id,
            &image.image_type,
            image.panel_id.as_deref(),
            None,
        )?;
        active.panel_id = Set(None);
    } else if let Some(panel_id) = input.panel_id {
        let panel_id = normalize_optional_video_record_text(Some(panel_id.as_str()));
        validate_generated_image_update_panel_binding_fields(
            image.id,
            &image.image_type,
            image.panel_id.as_deref(),
            panel_id.as_deref(),
        )?;
        active.panel_id = Set(panel_id);
    }
    if input.clear_ai_score {
        active.ai_score = Set(None);
    } else if let Some(score) = input.ai_score {
        active.ai_score = Set(Some(super::super::validate_generated_image_ai_score(
            score,
        )?));
    }
    if let Some(is_favorite) = input.is_favorite {
        active.is_favorite = Set(is_favorite);
    }
    active.update(db).await.map_err(VideoDbError::Database)
}
