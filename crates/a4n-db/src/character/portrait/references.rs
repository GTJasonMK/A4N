use super::CharacterPortraitRecordError;
use a4n_core::models::{
    ai::generated_image,
    character::{character_portrait, protagonist_profile},
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};
use std::collections::{HashMap, HashSet};

pub async fn missing_portrait_profiles<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(db)
        .await?;
    if profiles.is_empty() {
        return Ok(Vec::new());
    }
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let portrait_profile_ids = character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::ProfileId)
        .filter(character_portrait::Column::ProfileId.is_in(profile_ids))
        .group_by(character_portrait::Column::ProfileId)
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
    Ok(profiles
        .into_iter()
        .filter(|profile| !portrait_profile_ids.contains(&profile.id))
        .collect())
}

pub async fn portrait_reference_images_by_ids<C>(
    db: &C,
    project_id: i64,
    image_ids: &[i64],
) -> Result<Vec<generated_image::Model>, CharacterPortraitRecordError>
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
        ordered.push(image);
    }

    Ok(ordered)
}
