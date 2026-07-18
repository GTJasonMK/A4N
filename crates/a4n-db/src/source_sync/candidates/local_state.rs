use a4n_core::models::{
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
    video_project::assembly_clip,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::source_sync::{SourceSyncDbError, validate_positive_id};

pub async fn local_state_for_generation_spec<C>(
    db: &C,
    spec: &generation_spec::Model,
) -> Result<String, SourceSyncDbError>
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
        return Ok("user_edited".to_string());
    }
    let media = media_provenance::Entity::find()
        .filter(media_provenance::Column::GenerationSpecId.eq(spec.id))
        .all(db)
        .await?;
    for provenance in media {
        let Some(asset) = media_asset::Entity::find_by_id(provenance.media_asset_id)
            .one(db)
            .await?
        else {
            continue;
        };
        let clip_count = assembly_clip::Entity::find()
            .filter(assembly_clip::Column::MediaAssetId.eq(asset.id))
            .count(db)
            .await?;
        if clip_count > 0 {
            return Ok("assembled".to_string());
        }
        if asset.origin == "imported" {
            return Ok("imported".to_string());
        }
        if asset.status == "completed" {
            return Ok("generated".to_string());
        }
    }
    if spec.status == "draft" {
        Ok("draft".to_string())
    } else {
        Ok("clean".to_string())
    }
}
