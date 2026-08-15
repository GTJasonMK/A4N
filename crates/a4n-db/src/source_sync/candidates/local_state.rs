use a4n_core::models::{
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
    video_project::assembly_clip,
};
use a4n_domain::source_sync::SourceSyncLocalStateKind;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::source_sync::{SourceSyncDbError, validate_positive_id};

pub async fn local_state_for_generation_spec<C>(
    db: &C,
    spec: &generation_spec::Model,
) -> Result<SourceSyncLocalStateKind, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    let edited = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(spec.id))
        .filter(prompt_artifact::Column::EditedByUser.eq(true))
        .count(db)
        .await?;
    if edited > 0 {
        return Ok(SourceSyncLocalStateKind::UserEdited);
    }
    let media = media_provenance::Entity::find()
        .filter(media_provenance::Column::GenerationSpecId.eq(spec.id))
        .all(db)
        .await?;
    for provenance in media {
        let asset = media_asset::Entity::find_by_id(provenance.media_asset_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                SourceSyncDbError::NotFound(format!(
                    "媒体来源记录引用的媒体资产不存在: {}",
                    provenance.media_asset_id
                ))
            })?;
        let clip_count = assembly_clip::Entity::find()
            .filter(assembly_clip::Column::MediaAssetId.eq(asset.id))
            .count(db)
            .await?;
        if clip_count > 0 {
            return Ok(SourceSyncLocalStateKind::Assembled);
        }
        if asset.origin == "imported" {
            return Ok(SourceSyncLocalStateKind::Imported);
        }
        if asset.status == "completed" {
            return Ok(SourceSyncLocalStateKind::Generated);
        }
    }
    if spec.status == "draft" {
        Ok(SourceSyncLocalStateKind::Draft)
    } else {
        Ok(SourceSyncLocalStateKind::Clean)
    }
}
