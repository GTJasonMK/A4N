use super::contracts::ManifestIds;
use a4n_domain::canvas_transfer_runtime_identity_key;

use a4n_db_models::models::generation::generation_spec;

use crate::project::video_studio_package::VideoStudioProjectPackage;

use super::super::canvas::validate_generation_spec_target;

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
        if spec.source_context_kind.as_deref() != Some(generation_spec::SOURCE_CONTEXT_FREEFORM)
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
        let expected_source_version_hash =
            generation_spec::freeform_source_version_hash(&spec.params_json).ok_or_else(|| {
                format!(
                    "生成意图 params_json.source_material 必须是 JSON 对象: generation_spec_id={}",
                    spec.id
                )
            })?;
        if spec.source_version_hash != expected_source_version_hash {
            return Err(format!(
                "生成意图 source_version_hash 与 source_material 不一致: generation_spec_id={}",
                spec.id
            ));
        }
        if let Some(key) = canvas_transfer_runtime_identity_key(&spec.params_json) {
            return Err(format!(
                "生成意图 params_json 不能携带运行身份 {key}: generation_spec_id={}",
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
