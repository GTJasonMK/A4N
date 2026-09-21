use crate::galgame::{GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_TARGET_PORTRAIT_EXPRESSION};
use a4n_db_models::models::{
    generation::{generation_spec, prompt_artifact},
    media::media_provenance,
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashSet;

pub(super) async fn prompt_artifact_ids_for_generation_specs_record<C>(
    db: &C,
    generation_spec_ids: &[i64],
) -> Result<Vec<i64>, DbErr>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(Vec::new());
    }
    prompt_artifact::Entity::find()
        .select_only()
        .column(prompt_artifact::Column::Id)
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await
}

pub(super) async fn media_asset_ids_for_generation_specs_record<C>(
    db: &C,
    generation_spec_ids: &[i64],
) -> Result<Vec<i64>, DbErr>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(Vec::new());
    }
    media_provenance::Entity::find()
        .select_only()
        .column(media_provenance::Column::MediaAssetId)
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await
}

pub(super) async fn galgame_portrait_generation_spec_ids_for_source_lines_record<C>(
    db: &C,
    project_id: i64,
    line_ids: &[i64],
) -> Result<Vec<i64>, DbErr>
where
    C: ConnectionTrait,
{
    if line_ids.is_empty() {
        return Ok(Vec::new());
    }
    let line_ids = line_ids.iter().copied().collect::<HashSet<_>>();
    let rows = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .column(generation_spec::Column::ParamsJson)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetKind.eq(GALGAME_TARGET_PORTRAIT_EXPRESSION))
        .filter(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_CHARACTER_PROFILE))
        .into_tuple::<(i64, serde_json::Value)>()
        .all(db)
        .await?;
    let mut spec_ids = Vec::new();
    for (spec_id, params_json) in rows {
        if crate::galgame::galgame_portrait_spec_references_lines(&params_json, &line_ids)
            .map_err(|error| DbErr::Custom(format!("generation_spec {}: {error}", spec_id)))?
        {
            spec_ids.push(spec_id);
        }
    }
    Ok(spec_ids)
}
