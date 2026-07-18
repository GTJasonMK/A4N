use super::assets::delete_unbound_project_media_assets_record;
use super::queries::{
    galgame_portrait_generation_spec_ids_for_source_lines_record,
    media_asset_ids_for_generation_specs_record, prompt_artifact_ids_for_generation_specs_record,
};
use super::specs::delete_generation_spec_records_by_ids;
use crate::galgame::{GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE};
use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    media::media_binding,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect,
};

pub async fn delete_galgame_target_generation_media_graph_record<C>(
    db: &C,
    project_id: i64,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let script_ids = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if script_ids.is_empty() {
        return Ok(());
    }

    let scene_ids = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .filter(galgame_scene::Column::ScriptId.is_in(script_ids))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if scene_ids.is_empty() {
        return Ok(());
    }

    let line_ids = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::Id)
        .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await?;

    let mut spec_condition = Condition::any().add(
        Condition::all()
            .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_SCENE))
            .add(generation_spec::Column::TargetEntityId.is_in(scene_ids.clone())),
    );
    if !line_ids.is_empty() {
        spec_condition = spec_condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
                .add(generation_spec::Column::TargetEntityId.is_in(line_ids.clone())),
        );
    }

    let mut generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(spec_condition)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if !line_ids.is_empty() {
        generation_spec_ids.extend(
            galgame_portrait_generation_spec_ids_for_source_lines_record(db, project_id, &line_ids)
                .await?,
        );
        generation_spec_ids.sort_unstable();
        generation_spec_ids.dedup();
    }

    let prompt_artifact_ids =
        prompt_artifact_ids_for_generation_specs_record(db, &generation_spec_ids).await?;

    let mut binding_condition = Condition::any().add(
        Condition::all()
            .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_SCENE))
            .add(media_binding::Column::TargetId.is_in(scene_ids)),
    );
    if !line_ids.is_empty() {
        binding_condition = binding_condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
                .add(media_binding::Column::TargetId.is_in(line_ids)),
        );
    }

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
    delete_unbound_project_media_assets_record(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_spec_records_by_ids(db, generation_spec_ids, prompt_artifact_ids).await
}
