use a4n_db_models::models::{
    generation::generation_spec,
    media::{media_asset, media_provenance},
};

use super::super::contracts::MediaProvenanceRefContext;
use super::context::validate_media_provenance_source_context;

pub(super) fn validate_generation_spec_ref<'a>(
    provenance: &media_provenance::Model,
    asset: &media_asset::Model,
    refs: &MediaProvenanceRefContext<'a>,
) -> Result<Option<&'a generation_spec::Model>, String> {
    let Some(spec_id) = provenance.generation_spec_id else {
        return Ok(None);
    };
    let spec = refs
        .request
        .generation_specs
        .get(&spec_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包媒体来源 generation_spec_id 不存在: media_asset_id={}, generation_spec_id={spec_id}",
                provenance.media_asset_id
            )
        })?;
    if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
        return Err(format!(
            "项目族导入包媒体来源生成意图与媒体资产项目不一致: media_asset_id={}, generation_spec_id={spec_id}",
            provenance.media_asset_id
        ));
    }
    validate_media_provenance_source_context(
        provenance.media_asset_id,
        asset,
        spec.source_snapshot_id,
        spec.source_snapshot_chapter_id,
        spec.source_chapter_id,
        "generation_spec_id",
        spec.id,
    )?;
    Ok(Some(spec))
}
