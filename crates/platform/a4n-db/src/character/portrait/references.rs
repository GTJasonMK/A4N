use super::CharacterPortraitRecordError;
use a4n_db_models::models::{
    ai::generated_image,
    character::{character_portrait, protagonist_profile},
};
use a4n_domain::media::GeneratedImageRecord;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, sea_query::Query};
use std::collections::HashMap;

use crate::video::media_mapping::generated_image_record_from_model;

pub async fn missing_portrait_profiles<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let portrait_profile_ids = Query::select()
        .column(character_portrait::Column::ProfileId)
        .from(character_portrait::Entity)
        .and_where(character_portrait::Column::ProjectId.eq(project_id))
        .group_by_col(character_portrait::Column::ProfileId)
        .to_owned();
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Id.not_in_subquery(portrait_profile_ids))
        .all(db)
        .await
}

pub async fn portrait_reference_images_by_ids<C>(
    db: &C,
    project_id: i64,
    image_ids: &[i64],
) -> Result<Vec<GeneratedImageRecord>, CharacterPortraitRecordError>
where
    C: ConnectionTrait,
{
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }

    let images = generated_image::Entity::find()
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .all(db)
        .await
        .map_err(CharacterPortraitRecordError::Database)?;
    let mut by_id: HashMap<_, _> = images.into_iter().map(|image| (image.id, image)).collect();
    let mut ordered = Vec::with_capacity(image_ids.len());

    for image_id in image_ids {
        let image = by_id.remove(image_id).ok_or_else(|| {
            CharacterPortraitRecordError::NotFound(format!("参考图不存在：#{image_id}"))
        })?;
        if image.project_id != project_id {
            return Err(CharacterPortraitRecordError::InvalidArgument(format!(
                "参考图 #{} 不属于当前项目",
                image_id
            )));
        }
        ordered.push(generated_image_record_from_model(image));
    }

    Ok(ordered)
}
