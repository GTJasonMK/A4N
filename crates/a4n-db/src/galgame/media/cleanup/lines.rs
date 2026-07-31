use super::{
    delete_generation_media_graph,
    queries::galgame_portrait_generation_spec_ids_for_source_lines_record,
};
use crate::galgame::{GALGAME_ENTITY_LINE, GalgameLineRecordError, MEDIA_BINDING_ROLE_CG};
use a4n_core::models::{generation::generation_spec, media::media_binding};
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub async fn delete_galgame_line_visual_generation_media_graph_record<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
) -> Result<(), GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    delete_galgame_line_generation_media_graph(db, project_id, line_id, Some(MEDIA_BINDING_ROLE_CG))
        .await
}

pub async fn delete_galgame_line_generation_media_graph_record<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
) -> Result<(), GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    delete_galgame_line_generation_media_graph(db, project_id, line_id, None).await
}

async fn delete_galgame_line_generation_media_graph<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
    binding_role: Option<&str>,
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

    let mut binding_condition = Condition::all()
        .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .add(media_binding::Column::TargetId.eq(line_id));
    if let Some(role) = binding_role {
        binding_condition = binding_condition.add(media_binding::Column::Role.eq(role));
    }
    delete_generation_media_graph(db, project_id, generation_spec_ids, binding_condition)
        .await
        .map_err(GalgameLineRecordError::Database)
}
