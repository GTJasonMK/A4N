use super::super::*;
use super::contracts::ManifestIds;

pub(super) fn validate_generation_records(
    package: &VideoStudioProjectPackage,
    ids: &ManifestIds,
) -> Result<(), String> {
    for spec in &package.generation_specs {
        if spec.project_id != package.project.id || spec.root_project_id != package.project.id {
            return Err(format!(
                "生成意图不属于导入包项目: generation_spec_id={}",
                spec.id
            ));
        }
        if spec
            .source_context_kind
            .as_deref()
            .unwrap_or("snapshot_backed")
            != SOURCE_MODE_FREEFORM
            || spec.source_snapshot_id.is_some()
            || spec.source_chapter_id.is_some()
            || spec.source_snapshot_chapter_id.is_some()
        {
            return Err(format!(
                "视频画布项目包只允许 freeform 生成意图: generation_spec_id={}",
                spec.id
            ));
        }
        if let Some(asset_id) = spec.canvas_text_asset_id
            && !ids.text_asset_ids.contains(&asset_id)
        {
            return Err(format!(
                "生成意图引用了不存在的画布文本资产: generation_spec_id={}, canvas_text_asset_id={asset_id}",
                spec.id
            ));
        }
        validate_generation_spec_target(
            spec,
            &ids.node_ids,
            &ids.media_asset_ids,
            &ids.video_segment_ids,
            &ids.assembly_timeline_ids,
        )?;
    }
    for artifact in &package.prompt_artifacts {
        if !ids
            .generation_spec_ids
            .contains(&artifact.generation_spec_id)
        {
            return Err(format!(
                "提示词产物引用了不存在的生成意图: prompt_artifact_id={}, generation_spec_id={}",
                artifact.id, artifact.generation_spec_id
            ));
        }
    }
    Ok(())
}
