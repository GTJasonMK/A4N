use super::{CharacterPortraitRollbackError, CharacterPortraitRollbackResult};
use a4n_core::models::{
    ai::generated_image, asset::profile_primary_material, character::character_portrait,
    project::novel_project,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use serde_json::Value;

async fn rollback_failed_portrait_creation_records_in<C>(
    db: &C,
    portrait_id: i64,
    image_path: &str,
) -> Result<CharacterPortraitRollbackResult, CharacterPortraitRollbackError>
where
    C: ConnectionTrait,
{
    let mut portrait_query = character_portrait::Entity::find_by_id(portrait_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        portrait_query = portrait_query.lock_exclusive();
    }
    let portrait = portrait_query
        .select_only()
        .column(character_portrait::Column::ProjectId)
        .column(character_portrait::Column::ImagePath)
        .into_tuple::<(i64, String)>()
        .one(db)
        .await
        .map_err(CharacterPortraitRollbackError::QueryPortrait)?
        .ok_or_else(|| {
            CharacterPortraitRollbackError::NotFound(format!("立绘不存在: {portrait_id}"))
        })?;
    if portrait.1 != image_path {
        return Err(CharacterPortraitRollbackError::InvalidArgument(format!(
            "立绘 {portrait_id} 的图片路径与回滚目标不一致"
        )));
    }
    if db.get_database_backend() != DbBackend::Sqlite {
        novel_project::Entity::find_by_id(portrait.0)
            .select_only()
            .column(novel_project::Column::Id)
            .lock_exclusive()
            .into_tuple::<i64>()
            .one(db)
            .await
            .map_err(CharacterPortraitRollbackError::LockMirrorImages)?;
    }

    let mut image_query = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(portrait.0))
        .filter(generated_image::Column::FilePath.eq(&portrait.1))
        .order_by_asc(generated_image::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        image_query = image_query.lock_exclusive();
    }
    let image_ids = image_query
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::MetadataJson)
        .into_tuple::<(i64, Option<Value>)>()
        .all(db)
        .await
        .map_err(CharacterPortraitRollbackError::LockMirrorImages)?
        .into_iter()
        .filter(|(_, metadata_json)| {
            generated_image_metadata_matches_portrait(metadata_json.as_ref(), portrait_id)
        })
        .map(|(image_id, _)| image_id)
        .collect::<Vec<_>>();

    crate::video::clear_generated_image_references_record(db, &image_ids)
        .await
        .map_err(CharacterPortraitRollbackError::ClearMirrorImageReferences)?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::PrimaryImageId.is_in(image_ids.clone()))
        .exec(db)
        .await
        .map_err(CharacterPortraitRollbackError::DeleteMirrorImagePrimaryMaterials)?;
    crate::media::delete_generated_image_target_media_bindings(db, &image_ids)
        .await
        .map_err(CharacterPortraitRollbackError::DeleteMirrorImageBindings)?;
    crate::generation::delete_generation_run_targets_for(db, "generated_image", &image_ids)
        .await
        .map_err(CharacterPortraitRollbackError::DeleteMirrorImageTargets)?;

    generated_image::Entity::delete_many()
        .filter(generated_image::Column::Id.is_in(image_ids))
        .exec(db)
        .await
        .map_err(CharacterPortraitRollbackError::DeleteMirrorImages)?;

    crate::generation::delete_generation_run_targets_for(db, "character_portrait", &[portrait_id])
        .await
        .map_err(CharacterPortraitRollbackError::DeletePortraitTargets)?;

    let deleted = character_portrait::Entity::delete_by_id(portrait_id)
        .exec(db)
        .await
        .map_err(CharacterPortraitRollbackError::DeletePortrait)?;
    if deleted.rows_affected == 0 {
        return Err(CharacterPortraitRollbackError::NotFound(format!(
            "立绘不存在: {portrait_id}"
        )));
    }

    let delete_file = !crate::media::media_path_has_persisted_reference(db, image_path)
        .await
        .map_err(CharacterPortraitRollbackError::CheckFileReferences)?;
    Ok(CharacterPortraitRollbackResult { delete_file })
}

#[cfg(test)]
pub(in crate::character) fn generated_image_matches_portrait(
    image: &generated_image::Model,
    portrait_id: i64,
) -> bool {
    generated_image_metadata_matches_portrait(image.metadata_json.as_ref(), portrait_id)
}

pub(in crate::character) fn generated_image_metadata_matches_portrait(
    metadata_json: Option<&Value>,
    portrait_id: i64,
) -> bool {
    let metadata_portrait_id = metadata_json.and_then(|metadata| {
        metadata
            .get("portrait_id")
            .or_else(|| metadata.get("character_portrait_id"))
            .and_then(Value::as_i64)
    });
    metadata_portrait_id.is_none_or(|metadata_portrait_id| metadata_portrait_id == portrait_id)
}

pub async fn rollback_failed_portrait_creation_records<C>(
    db: &C,
    portrait_id: i64,
    image_path: &str,
) -> Result<CharacterPortraitRollbackResult, CharacterPortraitRollbackError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db
        .begin()
        .await
        .map_err(CharacterPortraitRollbackError::Begin)?;
    let result = rollback_failed_portrait_creation_records_in(&tx, portrait_id, image_path).await?;
    tx.commit()
        .await
        .map_err(CharacterPortraitRollbackError::Commit)?;
    Ok(result)
}

#[cfg(test)]
pub(in crate::character) fn generated_image_matches_any_portrait(
    image: &generated_image::Model,
    portrait_ids: &[i64],
) -> bool {
    generated_image_metadata_matches_any_portrait(image.metadata_json.as_ref(), portrait_ids)
}

pub(in crate::character) fn generated_image_metadata_matches_any_portrait(
    metadata_json: Option<&Value>,
    portrait_ids: &[i64],
) -> bool {
    let metadata_portrait_id = metadata_json.and_then(|metadata| {
        metadata
            .get("portrait_id")
            .or_else(|| metadata.get("character_portrait_id"))
            .and_then(Value::as_i64)
    });
    metadata_portrait_id
        .is_none_or(|metadata_portrait_id| portrait_ids.contains(&metadata_portrait_id))
}
