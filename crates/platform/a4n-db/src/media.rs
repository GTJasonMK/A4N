use std::collections::HashSet;

use a4n_db_models::models::{
    generation::generation_spec_media_ref,
    media::{media_asset, media_binding, media_provenance},
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};

mod bindings;
mod creation;
mod paths;
mod provenance;
mod records;
mod types;

pub(crate) use bindings::{
    delete_generated_image_target_media_bindings, delete_generated_video_target_media_bindings,
    generated_video_target_media_binding_project_ids,
};
pub use creation::create_media_asset_with_provenance_record;
pub(crate) use paths::media_path_has_persisted_reference;
pub use paths::{
    filter_unreferenced_media_paths, media_path_prefix_has_persisted_reference,
    project_has_active_image_publication, protected_image_publication_cleanup_paths,
};
pub use records::create_media_asset_with_provenance_read_model;
pub use types::MediaDbError;

pub(crate) async fn schedule_unreferenced_media_cleanup<C>(
    db: &C,
    project_id: i64,
    candidate_paths: &[String],
    dedupe_key: String,
) -> Result<Vec<String>, DbErr>
where
    C: ConnectionTrait,
{
    let paths = filter_unreferenced_media_paths(db, candidate_paths).await?;
    if !paths.is_empty() {
        crate::cleanup_outbox::insert_file_artifact_cleanup_intent(
            db,
            dedupe_key,
            crate::cleanup_outbox::FileArtifactCleanupPayload::new(project_id, paths.clone()),
        )
        .await?;
    }
    Ok(paths)
}

pub(crate) async fn delete_unbound_project_media_assets<C>(
    db: &C,
    project_id: i64,
    candidate_media_asset_ids: HashSet<i64>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if candidate_media_asset_ids.is_empty() {
        return Ok(());
    }
    let candidates = candidate_media_asset_ids
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let still_bound = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::MediaAssetId.is_in(candidates))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
    let delete_media_asset_ids = candidate_media_asset_ids
        .difference(&still_bound)
        .copied()
        .collect::<Vec<_>>();
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    let delete_media_asset_ids = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()))
        .exec(db)
        .await?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()),
        )
        .exec(db)
        .await?;
    media_asset::Entity::delete_many()
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .exec(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
