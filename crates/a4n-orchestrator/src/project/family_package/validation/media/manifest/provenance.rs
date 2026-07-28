use super::super::super::super::ProjectFamilyPackage;
use super::super::{
    contracts::MediaProvenanceRefContext, provenance::validate_media_provenance_refs,
};
use super::context::MediaManifestContext;

pub(super) fn validate_media_provenance(
    package: &ProjectFamilyPackage,
    context: &MediaManifestContext<'_>,
) -> Result<(), String> {
    let refs = MediaProvenanceRefContext {
        media_assets: &context.media_assets_by_id,
        request: context.request_refs(),
    };
    for provenance in &package.media_provenance {
        let asset = context
            .media_assets_by_id
            .get(&provenance.media_asset_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包媒体来源引用了不存在的媒体资产: media_asset_id={}",
                    provenance.media_asset_id
                )
            })?;
        validate_media_provenance_refs(provenance, asset, &refs)?;
    }
    Ok(())
}
