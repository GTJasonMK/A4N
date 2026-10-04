use a4n_db_models::models::{
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
};
use a4n_error::AppError;
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
    // Specs and media assets are independent; their dependent projections form the second
    // layer. Extract results in the old spec -> artifact -> asset -> provenance order.
    let (generation_specs_result, media_assets_result) = futures::join!(
        generation_spec::Entity::find()
            .select_only()
            .column(generation_spec::Column::Id)
            .column(generation_spec::Column::TargetKind)
            .filter(generation_spec::Column::ProjectId.is_in(family_project_ids.iter().copied()))
            .order_by_asc(generation_spec::Column::Id)
            .into_model::<GenerationSpecGraphRow>()
            .all(db),
        media_asset::Entity::find()
            .select_only()
            .column(media_asset::Column::Id)
            .column(media_asset::Column::MediaKind)
            .filter(media_asset::Column::ProjectId.is_in(family_project_ids.iter().copied()))
            .order_by_asc(media_asset::Column::Id)
            .into_model::<MediaAssetGraphRow>()
            .all(db),
    );
    let generation_specs = generation_specs_result.map_err(AppError::from)?;
    let media_assets = media_assets_result.map_err(AppError::from)?;
    let generation_spec_ids = generation_specs
        .iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let media_asset_ids = media_assets.iter().map(|row| row.id).collect::<Vec<_>>();
    let (prompt_artifacts_result, media_provenance_count_result) = futures::join!(
        async {
            if generation_spec_ids.is_empty() {
                Ok(Vec::new())
            } else {
                prompt_artifact::Entity::find()
                    .select_only()
                    .column(prompt_artifact::Column::PromptKind)
                    .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids))
                    .order_by_asc(prompt_artifact::Column::Id)
                    .into_model::<PromptArtifactGraphRow>()
                    .all(db)
                    .await
            }
        },
        async {
            if media_asset_ids.is_empty() {
                Ok(0)
            } else {
                media_provenance::Entity::find()
                    .filter(media_provenance::Column::MediaAssetId.is_in(media_asset_ids))
                    .count(db)
                    .await
            }
        },
    );
    let prompt_artifacts = prompt_artifacts_result.map_err(AppError::from)?;
    let media_provenance_count = media_provenance_count_result.map_err(AppError::from)?;

    Ok(GenerationAssetGraphRecords {
        generation_specs,
        prompt_artifacts,
        media_assets,
        media_provenance_count,
    })
}
