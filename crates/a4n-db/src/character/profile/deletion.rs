use std::collections::BTreeSet;

use a4n_core::models::{
    ai::generated_image,
    asset::profile_primary_material,
    character::{
        attribute_event, character_portrait, character_state_index, protagonist_behavior_record,
        protagonist_deletion_mark, protagonist_profile, snapshot_reference,
    },
    project::novel_project,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use crate::character::portrait;

use super::types::{CharacterProfileDeleteError, CharacterProfileDeletionSummary};

pub(crate) async fn delete_profile_cascade_in<C>(
    db: &C,
    profile_id: i64,
) -> Result<CharacterProfileDeletionSummary, CharacterProfileDeleteError>
where
    C: ConnectionTrait,
{
    let mut profile_query = protagonist_profile::Entity::find_by_id(profile_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        profile_query = profile_query.lock_exclusive();
    }
    let profile = profile_query
        .one(db)
        .await?
        .ok_or_else(|| CharacterProfileDeleteError::NotFound("主角档案不存在".into()))?;
    if db.get_database_backend() != DbBackend::Sqlite {
        novel_project::Entity::find_by_id(profile.project_id)
            .lock_exclusive()
            .one(db)
            .await?;
    }

    let state_index_ids = character_state_index::Entity::find()
        .select_only()
        .column(character_state_index::Column::Id)
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let attribute_event_ids = attribute_event::Entity::find()
        .select_only()
        .column(attribute_event::Column::Id)
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let mut portraits_query = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .order_by_asc(character_portrait::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        portraits_query = portraits_query.lock_exclusive();
    }
    let portraits = portraits_query.all(db).await?;
    let portrait_ids = portraits
        .iter()
        .map(|portrait| portrait.id)
        .collect::<Vec<_>>();
    let portrait_paths = portraits
        .iter()
        .map(|portrait| portrait.image_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if !portrait_paths.is_empty() {
        let mut image_query = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(profile.project_id))
            .filter(generated_image::Column::FilePath.is_in(portrait_paths.clone()))
            .order_by_asc(generated_image::Column::Id);
        if db.get_database_backend() != DbBackend::Sqlite {
            image_query = image_query.lock_exclusive();
        }
        let image_ids = image_query
            .all(db)
            .await?
            .into_iter()
            .filter(|image| portrait::generated_image_matches_any_portrait(image, &portrait_ids))
            .map(|image| image.id)
            .collect::<Vec<_>>();
        crate::video::clear_generated_image_references_record(db, &image_ids).await?;
        profile_primary_material::Entity::delete_many()
            .filter(profile_primary_material::Column::PrimaryImageId.is_in(image_ids.clone()))
            .exec(db)
            .await?;
        crate::media::delete_generated_image_target_media_bindings(db, &image_ids).await?;
        crate::generation::delete_generation_run_targets_for(db, "generated_image", &image_ids)
            .await?;
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::Id.is_in(image_ids))
            .exec(db)
            .await?;
    }

    crate::generation::delete_generation_run_targets_for(db, "character_portrait", &portrait_ids)
        .await?;

    protagonist_behavior_record::Entity::delete_many()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    protagonist_deletion_mark::Entity::delete_many()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    character_state_index::Entity::delete_many()
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    character_portrait::Entity::delete_many()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;

    crate::generation::delete_generation_run_targets_for(db, "protagonist_profile", &[profile_id])
        .await?;
    let deleted = protagonist_profile::Entity::delete_by_id(profile_id)
        .exec(db)
        .await?;
    if deleted.rows_affected == 0 {
        return Err(CharacterProfileDeleteError::NotFound(
            "主角档案不存在".into(),
        ));
    }
    let deletable_portrait_paths =
        crate::media::filter_unreferenced_media_paths(db, &portrait_paths).await?;

    Ok(CharacterProfileDeletionSummary {
        project_id: profile.project_id,
        profile_id,
        portrait_paths: deletable_portrait_paths,
        attribute_event_ids,
        state_index_ids,
    })
}

pub async fn delete_profile_cascade_transaction<C>(
    db: &C,
    profile_id: i64,
) -> Result<CharacterProfileDeletionSummary, CharacterProfileDeleteError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db
        .begin()
        .await
        .map_err(CharacterProfileDeleteError::Database)?;
    let summary = delete_profile_cascade_in(&tx, profile_id).await?;
    tx.commit()
        .await
        .map_err(CharacterProfileDeleteError::Database)?;
    Ok(summary)
}
