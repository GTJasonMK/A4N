use super::super::{ProjectCleanupDbError, ProjectGenerationMediaGraphCleanupResult};
use super::{
    links::delete_project_generated_artifact_links, rows::delete_project_generated_artifact_rows,
};
use crate::project::validate_positive_i64;
use a4n_db_models::models::{
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub(crate) async fn delete_project_generation_media_graph<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectGenerationMediaGraphCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let media_asset_ids = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let prompt_artifact_ids = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .select_only()
            .column(prompt_artifact::Column::Id)
            .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await?
    };

    let link_cleanup = delete_project_generated_artifact_links(
        db,
        project_id,
        &media_asset_ids,
        &generation_spec_ids,
        &prompt_artifact_ids,
    )
    .await?;
    let row_cleanup = delete_project_generated_artifact_rows(
        db,
        project_id,
        &[],
        &[],
        &media_asset_ids,
        &generation_spec_ids,
    )
    .await?;

    Ok(ProjectGenerationMediaGraphCleanupResult {
        deleted_generation_specs: row_cleanup.deleted_generation_specs,
        deleted_prompt_artifacts: link_cleanup.deleted_prompt_artifacts,
        deleted_media_assets: row_cleanup.deleted_media_assets,
        deleted_media_provenance: link_cleanup.deleted_media_provenance,
        deleted_media_bindings: link_cleanup.deleted_media_bindings,
    })
}
