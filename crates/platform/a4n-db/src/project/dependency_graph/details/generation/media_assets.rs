use std::collections::{HashMap, HashSet};

use a4n_db_models::models::{
    generation::prompt_artifact,
    media::{media_asset, media_provenance},
};
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::{
    db_record_result_with_related, db_related_record, lineage_ref, optional_prompt_sources,
    prompt_source,
};

pub(in crate::project::dependency_graph) async fn media_asset_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await
        .map_err(AppError::database)?;
    if assets.is_empty() {
        return Ok(Vec::new());
    }
    let provenances = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(assets.iter().map(|asset| asset.id)))
        .all(db)
        .await
        .map_err(AppError::database)?;
    let provenance_by_asset = provenances
        .iter()
        .map(|provenance| (provenance.media_asset_id, provenance))
        .collect::<HashMap<_, _>>();
    let artifact_ids = provenances
        .iter()
        .filter_map(|row| row.prompt_artifact_id)
        .collect::<HashSet<_>>();
    let artifacts = if artifact_ids.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .filter(prompt_artifact::Column::Id.is_in(artifact_ids.iter().copied()))
            .all(db)
            .await
            .map_err(AppError::database)?
    };
    let artifact_by_id = artifacts
        .iter()
        .map(|artifact| (artifact.id, artifact))
        .collect::<HashMap<_, _>>();

    Ok(assets
        .into_iter()
        .map(|asset| {
            let provenance = provenance_by_asset.get(&asset.id).copied();
            let artifact = provenance
                .and_then(|row| row.prompt_artifact_id)
                .and_then(|id| artifact_by_id.get(&id).copied());
            media_asset_record(asset, provenance, artifact, warnings)
        })
        .collect())
}

pub(super) fn media_asset_record(
    asset: media_asset::Model,
    provenance: Option<&media_provenance::Model>,
    artifact: Option<&prompt_artifact::Model>,
    graph_warnings: &mut Vec<String>,
) -> AssetLineageRecord {
    if provenance.is_none() {
        graph_warnings.push(format!("媒体资产 #{} 缺少 media_provenance。", asset.id));
    }
    let origin = media_asset_origin(&asset, provenance, artifact.is_some()).to_string();
    AssetLineageRecord {
        id: format!("media_asset:{}", asset.id),
        label: format!("{} #{}", asset.media_kind, asset.id),
        asset_kind: "media_asset".to_string(),
        asset_id: Some(asset.id),
        status: Some(asset.status.clone()),
        origin,
        source_refs: provenance
            .and_then(|row| row.generation_spec_id)
            .map(|id| {
                vec![lineage_ref(
                    &format!("generation_spec:{id}"),
                    "统一生成规格",
                    "text",
                    "generation_spec",
                    Some(id),
                )]
            })
            .unwrap_or_default(),
        prompt_sources: artifact
            .map(|artifact| {
                optional_prompt_sources(vec![prompt_source(
                    &format!("prompt_artifact_text:{}", artifact.id),
                    "prompt_artifact",
                    Some(&artifact.prompt_text),
                    Some(artifact.id),
                    Some(artifact.edited_by_user),
                )])
            })
            .unwrap_or_default(),
        media_sources: provenance
            .and_then(|row| row.source_media_asset_id)
            .map(|id| {
                vec![lineage_ref(
                    &format!("media_asset:{id}"),
                    "源媒体资产",
                    "media",
                    "media_asset",
                    Some(id),
                )]
            })
            .unwrap_or_default(),
        request_snapshot: provenance.and_then(|row| row.request_snapshot_json.clone()),
        input_status: if provenance.is_some() {
            "complete"
        } else {
            "missing"
        }
        .to_string(),
        generation_inputs: Vec::new(),
        result: db_record_result_with_related(
            "media_asset",
            &asset,
            provenance
                .map(|row| db_related_record("media_provenance", row))
                .into_iter()
                .chain(artifact.map(|row| db_related_record("prompt_artifact", row)))
                .collect(),
        ),
        warnings: if provenance.is_none() {
            vec!["未找到媒体来源记录。".to_string()]
        } else if provenance.and_then(|row| row.prompt_artifact_id).is_some() && artifact.is_none()
        {
            vec!["媒体来源指向的 prompt_artifact 不存在。".to_string()]
        } else {
            Vec::new()
        },
    }
}

fn media_asset_origin(
    asset: &media_asset::Model,
    provenance: Option<&media_provenance::Model>,
    has_prompt_artifact: bool,
) -> &'static str {
    match provenance {
        Some(row) if row.imported_from_user => "imported",
        Some(row) if row.source_media_asset_id.is_some() => "extracted_frame",
        Some(_) if has_prompt_artifact && asset.media_kind == "video" => "video_generated",
        Some(_) if has_prompt_artifact => "image_generated",
        Some(_) if asset.origin == "generated" && asset.media_kind == "video" => "video_generated",
        Some(_) if asset.origin == "generated" => "image_generated",
        Some(_) => "manual",
        None => "unknown",
    }
}
