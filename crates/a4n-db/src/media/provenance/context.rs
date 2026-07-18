use a4n_core::models::media::media_asset;

use crate::media::MediaDbError;

pub(super) fn validate_provenance_source_context(
    asset: &media_asset::Model,
    ref_source_snapshot_id: Option<i64>,
    ref_source_snapshot_chapter_id: Option<i64>,
    ref_source_chapter_id: Option<i64>,
    label: &str,
    ref_id: i64,
) -> Result<(), MediaDbError> {
    match asset.source_snapshot_id {
        Some(asset_snapshot_id) if ref_source_snapshot_id != Some(asset_snapshot_id) => {
            return Err(MediaDbError::InvalidArgument(format!(
                "媒体来源{label} {ref_id} 的 source_snapshot_id 与媒体资产不一致"
            )));
        }
        None if ref_source_snapshot_id.is_some() || ref_source_snapshot_chapter_id.is_some() => {
            return Err(MediaDbError::InvalidArgument(format!(
                "freeform 媒体资产不能引用带源快照上下文的{label} {ref_id}"
            )));
        }
        _ => {}
    }

    if let Some(asset_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && ref_source_snapshot_chapter_id != Some(asset_snapshot_chapter_id)
    {
        return Err(MediaDbError::InvalidArgument(format!(
            "媒体来源{label} {ref_id} 的 source_snapshot_chapter_id 与媒体资产不一致"
        )));
    }
    if let Some(asset_source_chapter_id) = asset.source_chapter_id
        && ref_source_chapter_id != Some(asset_source_chapter_id)
    {
        return Err(MediaDbError::InvalidArgument(format!(
            "媒体来源{label} {ref_id} 的 source_chapter_id 与媒体资产不一致"
        )));
    }
    Ok(())
}
