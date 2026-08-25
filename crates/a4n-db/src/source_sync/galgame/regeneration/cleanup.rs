use a4n_core::models::galgame::{galgame_choice, galgame_line};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::source_sync::galgame::{
    bound_media_asset_ids_for_galgame_scene, delete_media_binding_records_for_galgame_scene,
    delete_unbound_project_media_asset_records, galgame_generation_spec_ids_for_scene,
    galgame_portrait_generation_spec_ids_for_source_lines, media_asset_ids_for_generation_specs,
};
use crate::source_sync::{SourceSyncDbError, delete_generation_spec_records};

pub(super) struct GalgameSceneRegenerationCleanup {
    pub(super) old_line_ids: Vec<i64>,
    pub(super) deleted_generation_spec_count: usize,
    pub(super) deleted_portrait_generation_spec_count: usize,
}

pub(super) async fn cleanup_galgame_scene_regeneration_records<C>(
    db: &C,
    project_id: i64,
    scene_id: i64,
) -> Result<GalgameSceneRegenerationCleanup, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let old_line_ids = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::Id)
        .filter(galgame_line::Column::SceneId.eq(scene_id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let mut generation_spec_ids =
        galgame_generation_spec_ids_for_scene(db, project_id, scene_id, &old_line_ids).await?;
    let portrait_generation_spec_ids =
        galgame_portrait_generation_spec_ids_for_source_lines(db, project_id, &old_line_ids)
            .await?;
    let deleted_portrait_generation_spec_count = portrait_generation_spec_ids.len();
    generation_spec_ids.extend(portrait_generation_spec_ids);
    generation_spec_ids.sort_unstable();
    generation_spec_ids.dedup();

    let bound_media_asset_ids =
        bound_media_asset_ids_for_galgame_scene(db, scene_id, &old_line_ids).await?;
    let provenance_media_asset_ids =
        media_asset_ids_for_generation_specs(db, &generation_spec_ids).await?;

    if !old_line_ids.is_empty() {
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.is_in(old_line_ids.clone()))
            .exec(db)
            .await?;
    }
    delete_media_binding_records_for_galgame_scene(db, scene_id, &old_line_ids).await?;
    delete_unbound_project_media_asset_records(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_spec_records(db, &generation_spec_ids).await?;
    if !old_line_ids.is_empty() {
        galgame_line::Entity::delete_many()
            .filter(galgame_line::Column::Id.is_in(old_line_ids.clone()))
            .exec(db)
            .await?;
    }

    Ok(GalgameSceneRegenerationCleanup {
        old_line_ids,
        deleted_generation_spec_count: generation_spec_ids.len(),
        deleted_portrait_generation_spec_count,
    })
}
