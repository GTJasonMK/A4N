use super::assets::delete_unbound_project_media_assets_record;
use super::queries::{
    galgame_portrait_generation_spec_ids_for_source_lines_record,
    media_asset_ids_for_generation_specs_record, prompt_artifact_ids_for_generation_specs_record,
};
use super::specs::delete_generation_spec_records_by_ids;
use crate::galgame::{GALGAME_ENTITY_LINE, GalgameLineRecordError, MEDIA_BINDING_ROLE_CG};
use a4n_core::models::{generation::generation_spec, media::media_binding};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub async fn delete_galgame_line_visual_generation_media_graph_record<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
) -> Result<(), GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    let mut generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
        .filter(generation_spec::Column::TargetEntityId.eq(line_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    generation_spec_ids.extend(
        galgame_portrait_generation_spec_ids_for_source_lines_record(db, project_id, &[line_id])
            .await?,
    );
    generation_spec_ids.sort_unstable();
    generation_spec_ids.dedup();

    let prompt_artifact_ids =
        prompt_artifact_ids_for_generation_specs_record(db, &generation_spec_ids).await?;
    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .filter(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_CG))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .filter(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_CG))
        .exec(db)
        .await?;

    let provenance_media_asset_ids =
        media_asset_ids_for_generation_specs_record(db, &generation_spec_ids).await?;
    delete_unbound_project_media_assets_record(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_spec_records_by_ids(db, generation_spec_ids, prompt_artifact_ids)
        .await
        .map_err(GalgameLineRecordError::Database)
}

pub async fn delete_galgame_line_generation_media_graph_record<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
) -> Result<(), GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    let mut generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
        .filter(generation_spec::Column::TargetEntityId.eq(line_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    generation_spec_ids.extend(
        galgame_portrait_generation_spec_ids_for_source_lines_record(db, project_id, &[line_id])
            .await?,
    );
    generation_spec_ids.sort_unstable();
    generation_spec_ids.dedup();

    let prompt_artifact_ids =
        prompt_artifact_ids_for_generation_specs_record(db, &generation_spec_ids).await?;
    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .exec(db)
        .await?;

    let provenance_media_asset_ids =
        media_asset_ids_for_generation_specs_record(db, &generation_spec_ids).await?;
    delete_unbound_project_media_assets_record(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_spec_records_by_ids(db, generation_spec_ids, prompt_artifact_ids)
        .await
        .map_err(GalgameLineRecordError::Database)
}
