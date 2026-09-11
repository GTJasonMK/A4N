use super::super::{ProjectCleanupDbError, ProjectGeneratedArtifactLinkCleanupResult};
use crate::project::validate_positive_i64;
use a4n_db_models::models::{
    generation::{
        generation_spec_entity_ref, generation_spec_media_ref, generation_spec_text_anchor,
        prompt_artifact,
    },
    media::{media_binding, media_provenance},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(crate) async fn delete_project_generated_artifact_links<C>(
    db: &C,
    project_id: i64,
    media_asset_ids: &[i64],
    generation_spec_ids: &[i64],
    prompt_artifact_ids: &[i64],
) -> Result<ProjectGeneratedArtifactLinkCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let mut deleted_media_bindings = 0;
    let mut deleted_media_provenance = 0;
    let mut deleted_prompt_artifacts = 0;

    if !media_asset_ids.is_empty() {
        deleted_media_bindings = media_binding::Entity::delete_many()
            .filter(media_binding::Column::MediaAssetId.is_in(media_asset_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected;
        deleted_media_provenance = media_provenance::Entity::delete_many()
            .filter(media_provenance::Column::MediaAssetId.is_in(media_asset_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected;
    }
    if !generation_spec_ids.is_empty() {
        generation_spec_media_ref::Entity::delete_many()
            .filter(
                generation_spec_media_ref::Column::GenerationSpecId
                    .is_in(generation_spec_ids.to_vec()),
            )
            .exec(db)
            .await?;
        if !prompt_artifact_ids.is_empty() {
            deleted_prompt_artifacts = prompt_artifact::Entity::delete_many()
                .filter(prompt_artifact::Column::Id.is_in(prompt_artifact_ids.to_vec()))
                .filter(
                    prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()),
                )
                .exec(db)
                .await?
                .rows_affected;
        }
        generation_spec_text_anchor::Entity::delete_many()
            .filter(
                generation_spec_text_anchor::Column::GenerationSpecId
                    .is_in(generation_spec_ids.to_vec()),
            )
            .exec(db)
            .await?;
        generation_spec_entity_ref::Entity::delete_many()
            .filter(
                generation_spec_entity_ref::Column::GenerationSpecId
                    .is_in(generation_spec_ids.to_vec()),
            )
            .exec(db)
            .await?;
    }
    if !media_asset_ids.is_empty() {
        generation_spec_media_ref::Entity::delete_many()
            .filter(generation_spec_media_ref::Column::MediaAssetId.is_in(media_asset_ids.to_vec()))
            .exec(db)
            .await?;
    }

    Ok(ProjectGeneratedArtifactLinkCleanupResult {
        deleted_media_bindings,
        deleted_media_provenance,
        deleted_prompt_artifacts,
    })
}
