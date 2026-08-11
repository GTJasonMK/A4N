use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_core::models::generation::generation_spec;
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::BTreeSet;

pub async fn galgame_generation_spec_ids_for_scene<C>(
    db: &C,
    project_id: i64,
    scene_id: i64,
    line_ids: &[i64],
) -> Result<Vec<i64>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(project_id, "project_id")?;
    validate_positive_id(scene_id, "scene_id")?;
    for line_id in line_ids {
        validate_positive_id(*line_id, "line_id")?;
    }

    let mut condition = Condition::any().add(
        Condition::all()
            .add(generation_spec::Column::TargetEntityKind.eq("galgame_scene"))
            .add(generation_spec::Column::TargetEntityId.eq(scene_id)),
    );
    if !line_ids.is_empty() {
        condition = condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq("galgame_line"))
                .add(generation_spec::Column::TargetEntityId.is_in(line_ids.to_vec())),
        );
    }
    generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(condition)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(SourceSyncDbError::from)
}

pub async fn galgame_portrait_generation_spec_ids_for_source_lines<C>(
    db: &C,
    project_id: i64,
    line_ids: &[i64],
) -> Result<Vec<i64>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(project_id, "project_id")?;
    if line_ids.is_empty() {
        return Ok(Vec::new());
    }
    for line_id in line_ids {
        validate_positive_id(*line_id, "line_id")?;
    }

    let line_ids = line_ids.iter().copied().collect::<BTreeSet<_>>();
    let rows = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetKind.eq("portrait_expression"))
        .filter(generation_spec::Column::TargetEntityKind.eq("character_profile"))
        .all(db)
        .await?;
    let mut spec_ids = Vec::new();
    for spec in rows {
        if crate::galgame::galgame_portrait_spec_references_lines(&spec.params_json, &line_ids)
            .map_err(|error| {
                SourceSyncDbError::Internal(format!("generation_spec {}: {error}", spec.id))
            })?
        {
            spec_ids.push(spec.id);
        }
    }
    Ok(spec_ids)
}
