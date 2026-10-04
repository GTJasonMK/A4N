use super::super::contracts::MediaProvenanceRefContext;
use super::context::validate_media_provenance_source_context;

pub(super) fn validate_prompt_artifact_ref(
    provenance: &a4n_domain::project::ProjectFamilyMediaProvenanceRecord,
    asset: &a4n_domain::project::ProjectFamilyMediaAssetRecord,
    linked_spec: Option<&a4n_domain::project::ProjectFamilyGenerationSpecRecord>,
    refs: &MediaProvenanceRefContext<'_>,
) -> Result<(), String> {
    let Some(prompt_id) = provenance.prompt_artifact_id else {
        return Ok(());
    };
    let prompt = refs
        .request
        .prompt_artifacts
        .get(&prompt_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包媒体来源 prompt_artifact_id 不存在: media_asset_id={}, prompt_artifact_id={prompt_id}",
                provenance.media_asset_id
            )
        })?;
    if let Some(spec) = linked_spec
        && prompt.generation_spec_id != spec.id
    {
        return Err(format!(
            "项目族导入包媒体来源 prompt_artifact_id 与 generation_spec_id 不一致: media_asset_id={}, prompt_artifact_id={prompt_id}, generation_spec_id={}",
            provenance.media_asset_id, spec.id
        ));
    }
    if linked_spec.is_none() {
        let prompt_spec = refs
            .request
            .generation_specs
            .get(&prompt.generation_spec_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包媒体来源 prompt_artifact 关联的生成意图不存在: media_asset_id={}, prompt_artifact_id={prompt_id}, generation_spec_id={}",
                    provenance.media_asset_id, prompt.generation_spec_id
                )
            })?;
        validate_media_provenance_source_context(
            provenance.media_asset_id,
            asset,
            prompt_spec.source_snapshot_id,
            prompt_spec.source_snapshot_chapter_id,
            prompt_spec.source_chapter_id,
            "prompt_artifact.generation_spec_id",
            prompt_spec.id,
        )?;
    }
    Ok(())
}
