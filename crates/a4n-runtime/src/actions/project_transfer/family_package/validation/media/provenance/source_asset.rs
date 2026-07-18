use a4n_core::models::media::{media_asset, media_provenance};
use a4n_orchestrator::media::MEDIA_STATUS_COMPLETED;

use super::super::contracts::MediaProvenanceRefContext;
use super::context::validate_media_provenance_source_context;

pub(super) fn validate_source_media_asset_ref(
    provenance: &media_provenance::Model,
    asset: &media_asset::Model,
    refs: &MediaProvenanceRefContext<'_>,
) -> Result<(), String> {
    let Some(source_media_asset_id) = provenance.source_media_asset_id else {
        return Ok(());
    };
    let source_asset = refs
        .media_assets
        .get(&source_media_asset_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包媒体来源 source_media_asset_id 不存在: media_asset_id={}, source_media_asset_id={source_media_asset_id}",
                provenance.media_asset_id
            )
        })?;
    if source_asset.root_project_id != asset.root_project_id {
        return Err(format!(
            "项目族导入包媒体来源 source_media_asset_id 与媒体资产根项目不一致: media_asset_id={}, source_media_asset_id={source_media_asset_id}",
            provenance.media_asset_id
        ));
    }
    if source_asset.status != MEDIA_STATUS_COMPLETED || source_asset.file_path.is_none() {
        return Err(format!(
            "项目族导入包媒体来源 source_media_asset_id 不是可追溯的已完成媒体: media_asset_id={}, source_media_asset_id={source_media_asset_id}",
            provenance.media_asset_id
        ));
    }
    validate_media_provenance_source_context(
        provenance.media_asset_id,
        asset,
        source_asset.source_snapshot_id,
        source_asset.source_snapshot_chapter_id,
        source_asset.source_chapter_id,
        "source_media_asset_id",
        source_asset.id,
    )
}
