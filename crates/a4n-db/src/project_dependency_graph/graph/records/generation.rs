use a4n_core::{
    error::AppError,
    models::{
        generation::{generation_spec, prompt_artifact},
        media::{media_asset, media_provenance},
    },
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use super::{GenerationSpecGraphRow, MediaAssetGraphRow, PromptArtifactGraphRow};

pub(super) struct GenerationAssetGraphRecords {
    pub(super) generation_specs: Vec<GenerationSpecGraphRow>,
    pub(super) prompt_artifacts: Vec<PromptArtifactGraphRow>,
    pub(super) media_assets: Vec<MediaAssetGraphRow>,
    pub(super) media_provenance_count: u64,
}

pub(super) async fn load_generation_asset_graph_records(
    db: &impl ConnectionTrait,
    family_project_ids: &[i64],
) -> Result<GenerationAssetGraphRecords, AppError> {
    let generation_specs = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .column(generation_spec::Column::TargetKind)
        .filter(generation_spec::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(generation_spec::Column::Id)
        .into_model::<GenerationSpecGraphRow>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let prompt_artifacts = if generation_specs.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .select_only()
            .column(prompt_artifact::Column::PromptKind)
            .filter(
                prompt_artifact::Column::GenerationSpecId
                    .is_in(generation_specs.iter().map(|row| row.id)),
            )
            .order_by_asc(prompt_artifact::Column::Id)
            .into_model::<PromptArtifactGraphRow>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    let media_assets = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .column(media_asset::Column::MediaKind)
        .filter(media_asset::Column::ProjectId.is_in(family_project_ids.iter().copied()))
        .order_by_asc(media_asset::Column::Id)
        .into_model::<MediaAssetGraphRow>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let media_provenance_count = if media_assets.is_empty() {
        0
    } else {
        media_provenance::Entity::find()
            .filter(
                media_provenance::Column::MediaAssetId.is_in(media_assets.iter().map(|row| row.id)),
            )
            .count(db)
            .await
            .map_err(AppError::Database)?
    };

    Ok(GenerationAssetGraphRecords {
        generation_specs,
        prompt_artifacts,
        media_assets,
        media_provenance_count,
    })
}
