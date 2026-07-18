use a4n_core::models::media::media_asset;

pub(super) fn validate_media_provenance_source_context(
    media_asset_id: i64,
    asset: &media_asset::Model,
    ref_source_snapshot_id: Option<i64>,
    ref_source_snapshot_chapter_id: Option<i64>,
    ref_source_chapter_id: Option<i64>,
    label: &str,
    ref_id: i64,
) -> Result<(), String> {
    match asset.source_snapshot_id {
        Some(asset_snapshot_id) if ref_source_snapshot_id != Some(asset_snapshot_id) => {
            return Err(format!(
                "项目族导入包媒体来源 {label} 与媒体资产 source_snapshot_id 不一致: media_asset_id={media_asset_id}, {label}={ref_id}, asset_source_snapshot_id={asset_snapshot_id}, ref_source_snapshot_id={ref_source_snapshot_id:?}"
            ));
        }
        None if ref_source_snapshot_id.is_some() || ref_source_snapshot_chapter_id.is_some() => {
            return Err(format!(
                "项目族导入包 freeform 媒体来源不能带源快照上下文: media_asset_id={media_asset_id}, {label}={ref_id}"
            ));
        }
        _ => {}
    }

    if let Some(asset_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && ref_source_snapshot_chapter_id != Some(asset_snapshot_chapter_id)
    {
        return Err(format!(
            "项目族导入包媒体来源 {label} 与媒体资产 source_snapshot_chapter_id 不一致: media_asset_id={media_asset_id}, {label}={ref_id}, asset_source_snapshot_chapter_id={asset_snapshot_chapter_id}, ref_source_snapshot_chapter_id={ref_source_snapshot_chapter_id:?}"
        ));
    }
    if let Some(asset_source_chapter_id) = asset.source_chapter_id
        && ref_source_chapter_id != Some(asset_source_chapter_id)
    {
        return Err(format!(
            "项目族导入包媒体来源 {label} 与媒体资产 source_chapter_id 不一致: media_asset_id={media_asset_id}, {label}={ref_id}, asset_source_chapter_id={asset_source_chapter_id}, ref_source_chapter_id={ref_source_chapter_id:?}"
        ));
    }
    Ok(())
}
