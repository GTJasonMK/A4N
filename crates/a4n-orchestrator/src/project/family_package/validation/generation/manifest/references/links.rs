use crate::media::MEDIA_STATUS_COMPLETED;

use crate::project::{ProjectFamilyPackage, snapshot_profile_key};

use super::super::super::targets::required_generation_spec_source_snapshot_id;
use super::context::GenerationReferenceContext;

pub(super) fn validate_generation_links(
    package: &ProjectFamilyPackage,
    refs: &GenerationReferenceContext<'_>,
) -> Result<(), String> {
    for record in &package.generation_spec_text_anchors {
        let spec = refs
            .specs
            .get(&record.generation_spec_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包生成意图文本锚点引用了不存在的生成意图: generation_spec_id={}, text_anchor_id={}",
                    record.generation_spec_id, record.text_anchor_id
                )
            })?;
        let anchor = refs
            .text_anchors
            .get(&record.text_anchor_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包生成意图文本锚点引用了不存在的文本锚点: generation_spec_id={}, text_anchor_id={}",
                    record.generation_spec_id, record.text_anchor_id
                )
            })?;
        let source_snapshot_id = required_generation_spec_source_snapshot_id(spec)?;
        if anchor.source_snapshot_id != Some(source_snapshot_id) {
            return Err(format!(
                "项目族导入包生成意图文本锚点不属于源快照: generation_spec_id={}, text_anchor_id={}, source_snapshot_id={:?}, expected={}",
                spec.id, anchor.id, anchor.source_snapshot_id, source_snapshot_id
            ));
        }
        if let Some(source_snapshot_chapter_id) = spec.source_snapshot_chapter_id
            && anchor.source_snapshot_chapter_id != Some(source_snapshot_chapter_id)
        {
            return Err(format!(
                "项目族导入包生成意图文本锚点不属于生成意图源快照章节: generation_spec_id={}, text_anchor_id={}, source_snapshot_chapter_id={:?}, expected={}",
                spec.id, anchor.id, anchor.source_snapshot_chapter_id, source_snapshot_chapter_id
            ));
        }
    }
    for record in &package.generation_spec_entity_refs {
        let spec = refs
            .specs
            .get(&record.generation_spec_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包生成意图实体引用了不存在的生成意图: generation_spec_id={}, entity_kind={}, entity_id={}",
                    record.generation_spec_id, record.entity_kind, record.entity_id
                )
            })?;
        let source_snapshot_id = required_generation_spec_source_snapshot_id(spec)?;
        if !refs.snapshot_profiles.contains_key(&(
            source_snapshot_id,
            snapshot_profile_key(&record.entity_kind, record.entity_id),
        )) {
            return Err(format!(
                "项目族导入包生成意图实体引用不属于源快照: generation_spec_id={}, entity_kind={}, entity_id={}, source_snapshot_id={}",
                spec.id, record.entity_kind, record.entity_id, source_snapshot_id
            ));
        }
    }
    for record in &package.generation_spec_media_refs {
        let spec = refs
            .specs
            .get(&record.generation_spec_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包生成意图媒体引用了不存在的生成意图: generation_spec_id={}, media_asset_id={}",
                    record.generation_spec_id, record.media_asset_id
                )
            })?;
        let asset = refs
            .media_assets
            .get(&record.media_asset_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包生成意图媒体引用了不存在的媒体资产: generation_spec_id={}, media_asset_id={}",
                    record.generation_spec_id, record.media_asset_id
                )
            })?;
        if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
            return Err(format!(
                "项目族导入包生成意图媒体引用跨项目: generation_spec_id={}, media_asset_id={}",
                spec.id, asset.id
            ));
        }
        if asset.status != MEDIA_STATUS_COMPLETED || asset.file_path.is_none() {
            return Err(format!(
                "项目族导入包生成意图媒体引用不是可用已完成资产: generation_spec_id={}, media_asset_id={}",
                spec.id, asset.id
            ));
        }
    }
    for artifact in &package.prompt_artifacts {
        if !refs.specs.contains_key(&artifact.generation_spec_id) {
            return Err(format!(
                "项目族导入包提示词引用了不存在的生成意图: prompt_artifact_id={}, generation_spec_id={}",
                artifact.id, artifact.generation_spec_id
            ));
        }
    }
    Ok(())
}
