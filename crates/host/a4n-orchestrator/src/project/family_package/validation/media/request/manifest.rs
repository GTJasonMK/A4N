use serde_json::{Map, Value};

use super::super::contracts::MediaRequestRefContext;
use super::{
    fields::{
        optional_media_request_i64, required_media_request_i64, required_media_request_string,
        validate_optional_media_request_id_ref,
    },
    galgame::validate_media_request_galgame_target_line,
    is_galgame_visual_generation_spec,
    targets::validate_media_request_target_entity,
};

pub(in super::super) fn validate_media_request_snapshot_refs(
    provenance: &a4n_domain::project::ProjectFamilyMediaProvenanceRecord,
    asset: &a4n_domain::project::ProjectFamilyMediaAssetRecord,
    linked_spec: Option<&a4n_domain::project::ProjectFamilyGenerationSpecRecord>,
    request: &Map<String, Value>,
    refs: &MediaRequestRefContext<'_>,
) -> Result<(), String> {
    validate_optional_media_request_id_ref(
        request,
        "generation_spec_id",
        refs.generation_spec_ids,
        "生成意图",
        provenance.media_asset_id,
    )?;
    validate_optional_media_request_id_ref(
        request,
        "prompt_artifact_id",
        refs.prompt_artifact_ids,
        "提示词产物",
        provenance.media_asset_id,
    )?;
    validate_optional_media_request_id_ref(
        request,
        "timeline_id",
        refs.assembly_timeline_ids,
        "成片时间线",
        provenance.media_asset_id,
    )?;

    let request_source = request.get("source").and_then(Value::as_str).map(str::trim);
    if request_source == Some("galgame_audio_import") {
        let target_line_id =
            required_media_request_i64(request, "target_line_id", provenance.media_asset_id)?;
        validate_media_request_galgame_target_line(
            provenance.media_asset_id,
            asset,
            target_line_id,
            refs.galgame_scripts,
            refs.galgame_scenes,
            refs.galgame_lines,
        )?;
    } else if let Some(target_line_id) =
        optional_media_request_i64(request, "target_line_id", provenance.media_asset_id)?
    {
        validate_media_request_galgame_target_line(
            provenance.media_asset_id,
            asset,
            target_line_id,
            refs.galgame_scripts,
            refs.galgame_scenes,
            refs.galgame_lines,
        )?;
    }

    let is_galgame_asset_generation = request_source == Some("galgame_asset_generation");
    if linked_spec.is_some_and(is_galgame_visual_generation_spec) && !is_galgame_asset_generation {
        return Err(format!(
            "项目族导入包 Galgame 资产媒体来源 request_snapshot_json.source 必须是 galgame_asset_generation: media_asset_id={}",
            provenance.media_asset_id
        ));
    }

    if is_galgame_asset_generation {
        let spec = linked_spec.ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 资产媒体来源缺少 generation_spec_id: media_asset_id={}",
                provenance.media_asset_id
            )
        })?;
        let target_kind =
            required_media_request_string(request, "target_kind", provenance.media_asset_id)?;
        let target_entity_kind = required_media_request_string(
            request,
            "target_entity_kind",
            provenance.media_asset_id,
        )?;
        let target_entity_id =
            required_media_request_i64(request, "target_entity_id", provenance.media_asset_id)?;
        validate_media_request_target_entity(
            provenance.media_asset_id,
            asset,
            target_entity_kind,
            target_entity_id,
            refs,
        )?;
        if target_kind != spec.target_kind
            || target_entity_kind != spec.target_entity_kind
            || spec.target_entity_id != Some(target_entity_id)
        {
            return Err(format!(
                "项目族导入包 Galgame 资产媒体来源 request_snapshot_json 与 generation_spec 目标不一致: media_asset_id={}, generation_spec_id={}",
                provenance.media_asset_id, spec.id
            ));
        }
    } else if let Some(target_entity_id) =
        optional_media_request_i64(request, "target_entity_id", provenance.media_asset_id)?
    {
        let target_entity_kind = required_media_request_string(
            request,
            "target_entity_kind",
            provenance.media_asset_id,
        )?;
        validate_media_request_target_entity(
            provenance.media_asset_id,
            asset,
            target_entity_kind,
            target_entity_id,
            refs,
        )?;
    }
    Ok(())
}
