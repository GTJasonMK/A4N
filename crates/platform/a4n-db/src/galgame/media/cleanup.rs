mod assets;
mod lines;
mod project;
mod queries;
mod scenes;

use a4n_db_models::models::media::media_binding;
use sea_orm::{Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashSet;

use self::queries::{
    media_asset_ids_for_generation_specs_record, prompt_artifact_ids_for_generation_specs_record,
};
use crate::{
    generation::delete_generation_spec_records_by_ids, media::delete_unbound_project_media_assets,
};

pub use assets::delete_media_asset_with_provenance_records;
pub(in crate::galgame) use lines::{
    delete_galgame_line_generation_media_graph_record,
    delete_galgame_line_visual_generation_media_graph_record,
};
pub(crate) use project::delete_galgame_target_generation_media_graph_record;
pub(in crate::galgame) use scenes::delete_galgame_scene_visual_generation_media_graph_record;

async fn delete_generation_media_graph<C>(
    db: &C,
    project_id: i64,
    generation_spec_ids: Vec<i64>,
    binding_condition: Condition,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let prompt_artifact_ids =
        prompt_artifact_ids_for_generation_specs_record(db, &generation_spec_ids).await?;
    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(binding_condition.clone())
        .into_tuple::<i64>()
        .all(db)
        .await?;
    media_binding::Entity::delete_many()
        .filter(binding_condition)
        .exec(db)
        .await?;
    let provenance_media_asset_ids =
        media_asset_ids_for_generation_specs_record(db, &generation_spec_ids).await?;
    delete_unbound_project_media_assets(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect::<HashSet<_>>(),
    )
    .await?;
    delete_generation_spec_records_by_ids(db, generation_spec_ids, prompt_artifact_ids).await
}

#[cfg(test)]
mod tests;
