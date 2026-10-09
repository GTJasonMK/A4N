use super::{
    delete_generation_media_graph,
    queries::galgame_portrait_generation_spec_ids_for_source_lines_record,
};
use crate::galgame::{GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE};
use a4n_db_models::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    media::media_binding,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect,
};

pub(crate) async fn delete_galgame_target_generation_media_graph_record<C>(
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

    delete_generation_media_graph(db, project_id, generation_spec_ids, binding_condition).await
}
