use a4n_db_models::models::{
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
    video_project::assembly_clip,
};
use a4n_domain::source_sync::SourceSyncLocalStateKind;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::{HashMap, HashSet};

use crate::source_sync::{SourceSyncDbError, validate_positive_id};

pub async fn local_states_for_generation_specs<C>(
    db: &C,
    specs: &[generation_spec::Model],
) -> Result<HashMap<i64, SourceSyncLocalStateKind>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    for spec in specs {
        validate_positive_id(spec.id, "generation_spec_id")?;
    }
    if specs.is_empty() {
        return Ok(HashMap::new());
    }

    let spec_ids = specs.iter().map(|spec| spec.id).collect::<HashSet<_>>();
    let edited = prompt_artifact::Entity::find()
        .select_only()
        .column(prompt_artifact::Column::GenerationSpecId)
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(spec_ids.iter().copied()))
        .filter(prompt_artifact::Column::EditedByUser.eq(true))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
    let mut states = edited
        .iter()
        .map(|spec_id| (*spec_id, SourceSyncLocalStateKind::UserEdited))
        .collect::<HashMap<_, _>>();
    let active_specs = specs
        .iter()
        .filter(|spec| !edited.contains(&spec.id))
        .collect::<Vec<_>>();
    if active_specs.is_empty() {
        return Ok(states);
    }
    let active_spec_ids = active_specs
        .iter()
        .map(|spec| spec.id)
        .collect::<HashSet<_>>();
    let media = media_provenance::Entity::find()
        .select_only()
        .column(media_provenance::Column::MediaAssetId)
        .column(media_provenance::Column::GenerationSpecId)
        .filter(media_provenance::Column::GenerationSpecId.is_in(active_spec_ids.iter().copied()))
        .into_tuple::<(i64, Option<i64>)>()
        .all(db)
        .await?;
    let mut media_asset_ids = HashSet::new();
    let mut media_by_spec = HashMap::<i64, Vec<i64>>::new();
    for (media_asset_id, generation_spec_id) in media {
        if let Some(spec_id) = generation_spec_id {
            media_asset_ids.insert(media_asset_id);
            media_by_spec
                .entry(spec_id)
                .or_default()
                .push(media_asset_id);
        }
    }
    let (assets_by_id, assembled_asset_ids) = futures::join!(
        async {
            if media_asset_ids.is_empty() {
                Ok::<HashMap<i64, (i64, String, String)>, SourceSyncDbError>(HashMap::new())
            } else {
                Ok(media_asset::Entity::find()
                    .select_only()
                    .column(media_asset::Column::Id)
                    .column(media_asset::Column::Origin)
                    .column(media_asset::Column::Status)
                    .filter(media_asset::Column::Id.is_in(media_asset_ids.iter().copied()))
                    .into_tuple::<(i64, String, String)>()
                    .all(db)
                    .await?
                    .into_iter()
                    .map(|asset| (asset.0, asset))
                    .collect::<HashMap<_, _>>())
            }
        },
        async {
            if media_asset_ids.is_empty() {
                Ok::<HashSet<i64>, SourceSyncDbError>(HashSet::new())
            } else {
                Ok(assembly_clip::Entity::find()
                    .select_only()
                    .column(assembly_clip::Column::MediaAssetId)
                    .filter(
                        assembly_clip::Column::MediaAssetId.is_in(media_asset_ids.iter().copied()),
                    )
                    .into_tuple::<i64>()
                    .all(db)
                    .await?
                    .into_iter()
                    .collect::<HashSet<_>>())
            }
        },
    );
    let assets_by_id = assets_by_id?;
    let assembled_asset_ids = assembled_asset_ids?;

    for spec in active_specs {
        let mut state = None;
        if let Some(media) = media_by_spec.get(&spec.id) {
            for media_asset_id in media {
                let asset = assets_by_id.get(media_asset_id).ok_or_else(|| {
                    SourceSyncDbError::NotFound(format!(
                        "媒体来源记录引用的媒体资产不存在: {}",
                        media_asset_id
                    ))
                })?;
                if assembled_asset_ids.contains(media_asset_id) {
                    state = Some(SourceSyncLocalStateKind::Assembled);
                    break;
                }
                if asset.1 == "imported" {
                    state = Some(SourceSyncLocalStateKind::Imported);
                    break;
                }
                if asset.2 == "completed" {
                    state = Some(SourceSyncLocalStateKind::Generated);
                    break;
                }
            }
        }
        states.insert(
            spec.id,
            state.unwrap_or_else(|| {
                if spec.status == "draft" {
                    SourceSyncLocalStateKind::Draft
                } else {
                    SourceSyncLocalStateKind::Clean
                }
            }),
        );
    }
    Ok(states)
}
