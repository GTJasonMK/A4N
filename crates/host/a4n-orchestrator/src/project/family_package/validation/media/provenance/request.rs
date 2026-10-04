use super::super::contracts::MediaProvenanceRefContext;
use super::super::request::{
    is_galgame_visual_generation_spec, validate_media_request_snapshot_refs,
};

pub(super) fn validate_request_snapshot(
    provenance: &a4n_domain::project::ProjectFamilyMediaProvenanceRecord,
    asset: &a4n_domain::project::ProjectFamilyMediaAssetRecord,
    linked_spec: Option<&a4n_domain::project::ProjectFamilyGenerationSpecRecord>,
    refs: &MediaProvenanceRefContext<'_>,
) -> Result<(), String> {
    let Some(request_snapshot) = provenance.request_snapshot_json.as_ref() else {
        if linked_spec.is_some_and(is_galgame_visual_generation_spec) {
            return Err(format!(
                "项目族导入包 Galgame 资产媒体来源缺少 request_snapshot_json: media_asset_id={}",
                provenance.media_asset_id
            ));
        }
        return Ok(());
    };
    let request = request_snapshot.as_object().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json 必须是 JSON 对象: media_asset_id={}",
            provenance.media_asset_id
        )
    })?;
    validate_media_request_snapshot_refs(provenance, asset, linked_spec, request, &refs.request)
}
