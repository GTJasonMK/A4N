use super::delete_generation_media_graph;
use crate::galgame::{
    GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE, MEDIA_BINDING_ROLE_BACKGROUND, MEDIA_BINDING_ROLE_CG,
};
use a4n_db_models::models::{
    galgame::galgame_line, generation::generation_spec, media::media_binding,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect,
};

pub(in crate::galgame) async fn delete_galgame_scene_visual_generation_media_graph_record<C>(
    db: &C,
    project_id: i64,
    scene_id: i64,
    delete_background: bool,
    delete_line_cgs: bool,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if !delete_background && !delete_line_cgs {
        return Ok(());
    }
    let line_ids = if delete_line_cgs {
        galgame_line::Entity::find()
            .select_only()
            .column(galgame_line::Column::Id)
            .filter(galgame_line::Column::SceneId.eq(scene_id))
            .into_tuple::<i64>()
            .all(db)
            .await?
    } else {
        Vec::new()
    };
    if !delete_background && line_ids.is_empty() {
        return Ok(());
    }

    let mut spec_condition = Condition::any();
    if delete_background {
        spec_condition = spec_condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_SCENE))
                .add(generation_spec::Column::TargetEntityId.eq(scene_id)),
        );
    }
    if delete_line_cgs && !line_ids.is_empty() {
        spec_condition = spec_condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
                .add(generation_spec::Column::TargetEntityId.is_in(line_ids.clone())),
        );
    }
    let generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(spec_condition)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let mut binding_condition = Condition::any();
    if delete_background {
        binding_condition = binding_condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_SCENE))
                .add(media_binding::Column::TargetId.eq(scene_id))
                .add(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_BACKGROUND)),
        );
    }
    if delete_line_cgs && !line_ids.is_empty() {
        binding_condition = binding_condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
                .add(media_binding::Column::TargetId.is_in(line_ids))
                .add(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_CG)),
        );
    }
    delete_generation_media_graph(db, project_id, generation_spec_ids, binding_condition).await
}
