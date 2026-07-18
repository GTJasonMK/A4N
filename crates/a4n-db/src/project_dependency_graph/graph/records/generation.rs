use a4n_core::{
    error::AppError,
    models::{
        generation::{generation_spec, prompt_artifact},
        media::{media_asset, media_provenance},
    },
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(super) struct GenerationAssetGraphRecords {
    pub(super) generation_specs: Vec<generation_spec::Model>,
    pub(super) prompt_artifacts: Vec<prompt_artifact::Model>,
    pub(super) media_assets: Vec<media_asset::Model>,
    pub(super) media_provenance: Vec<media_provenance::Model>,
}

pub(super) async fn load_generation_asset_graph_records(
    db: &impl ConnectionTrait,
    family_project_ids: &[i64],
) -> Result<GenerationAssetGraphRecords, AppError> {
    let generation_specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let prompt_artifacts = if generation_specs.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .filter(
                prompt_artifact::Column::GenerationSpecId
                    .is_in(generation_specs.iter().map(|row| row.id)),
            )
            .order_by_asc(prompt_artifact::Column::Id)
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    let media_assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let media_provenance = if media_assets.is_empty() {
        Vec::new()
    } else {
        media_provenance::Entity::find()
            .filter(
                media_provenance::Column::MediaAssetId.is_in(media_assets.iter().map(|row| row.id)),
            )
            .order_by_asc(media_provenance::Column::MediaAssetId)
            .all(db)
            .await
            .map_err(AppError::Database)?
    };

    Ok(GenerationAssetGraphRecords {
        generation_specs,
        prompt_artifacts,
        media_assets,
        media_provenance,
    })
}
