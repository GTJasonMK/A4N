use super::super::types::{
    DeferredVideoStudioGenerationSpecTarget, InitialVideoStudioGenerationSpecTarget,
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps,
};
use super::ids::remap_video_studio_required_id;
use crate::generation::GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE;
use a4n_core::models::generation::generation_spec;

#[cfg(test)]
pub(in crate::canvas::transfer) fn remap_video_studio_generation_spec_target_entity_id(
    spec: &generation_spec::Model,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<Option<i64>, VideoStudioProjectImportError> {
    let Some(old_id) = spec.target_entity_id else {
        return Ok(None);
    };
    remap_video_studio_generation_spec_target_entity_id_parts(
        spec.id,
        &spec.target_entity_kind,
        old_id,
        maps,
    )
    .map(Some)
}

pub(in crate::canvas::transfer) fn initial_video_studio_generation_spec_target_entity_id(
    spec: &generation_spec::Model,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<InitialVideoStudioGenerationSpecTarget, VideoStudioProjectImportError> {
    let Some(old_id) = spec.target_entity_id else {
        return Ok(InitialVideoStudioGenerationSpecTarget {
            target_entity_id: None,
            deferred: None,
        });
    };
    match spec.target_entity_kind.as_str() {
        GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE => {
            Ok(InitialVideoStudioGenerationSpecTarget {
                target_entity_id: Some(remap_video_studio_required_id(
                    &maps.node_ids,
                    old_id,
                    "generation_spec.target_entity_id",
                )?),
                deferred: None,
            })
        }
        "media_asset" | "video_segment" | "assembly_timeline" => {
            Ok(InitialVideoStudioGenerationSpecTarget {
                target_entity_id: None,
                deferred: Some(DeferredVideoStudioGenerationSpecTarget {
                    target_entity_kind: spec.target_entity_kind.clone(),
                    old_target_entity_id: old_id,
                }),
            })
        }
        other => Err(unsupported_target_error(spec.id, other)),
    }
}

pub(in crate::canvas::transfer) fn remap_video_studio_generation_spec_target_entity_id_parts(
    spec_id: i64,
    target_entity_kind: &str,
    old_id: i64,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<i64, VideoStudioProjectImportError> {
    let map = match target_entity_kind {
        GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE => &maps.node_ids,
        "media_asset" => &maps.media_asset_ids,
        "video_segment" => &maps.video_segment_ids,
        "assembly_timeline" => &maps.assembly_timeline_ids,
        other => return Err(unsupported_target_error(spec_id, other)),
    };
    remap_video_studio_required_id(map, old_id, "generation_spec.target_entity_id")
}

fn unsupported_target_error(
    spec_id: i64,
    target_entity_kind: &str,
) -> VideoStudioProjectImportError {
    VideoStudioProjectImportError::InvalidPackage(format!(
        "视频画布项目包暂不支持生成意图目标类型: generation_spec_id={spec_id}, target_entity_kind={target_entity_kind}"
    ))
}
