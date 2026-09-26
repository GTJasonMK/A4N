use crate::media::{is_valid_media_binding_role, is_valid_media_binding_target_kind};

use super::super::super::super::ProjectFamilyPackage;
use super::super::bindings::{
    validate_galgame_media_binding_contract, validate_media_binding_target_ref,
};
use super::context::MediaManifestContext;

pub(super) fn validate_media_bindings(
    package: &ProjectFamilyPackage,
    context: &MediaManifestContext<'_>,
) -> Result<(), String> {
    for binding in &package.media_bindings {
        let asset = context.media_assets_by_id.get(&binding.media_asset_id);
        if asset.is_none() {
            return Err(format!(
                "项目族导入包媒体绑定引用了不存在的媒体资产: media_binding_id={}, media_asset_id={}",
                binding.id, binding.media_asset_id
            ));
        }
        if !is_valid_media_binding_target_kind(&binding.target_kind) {
            return Err(format!(
                "项目族导入包媒体绑定目标类型无效: media_binding_id={}, target_kind={}",
                binding.id, binding.target_kind
            ));
        }
        if !is_valid_media_binding_role(&binding.role) {
            return Err(format!(
                "项目族导入包媒体绑定角色无效: media_binding_id={}, role={}",
                binding.id, binding.role
            ));
        }
        validate_media_binding_target_ref(
            binding,
            &context.chapter_ids,
            &context.character_profile_ids,
            &context.object_profile_ids,
            &context.scene_profile_ids,
            &context.chapter_video_prompt_ids,
            &context.generated_video_ids,
            &context.video_segment_ids,
            &context.assembly_timeline_ids,
            &context.galgame_scene_ids,
            &context.galgame_line_ids,
        )?;
        validate_galgame_media_binding_contract(
            binding,
            asset.expect("checked media asset exists"),
            &context.galgame_scripts,
            &context.galgame_scenes,
            &context.galgame_lines,
            &context.source_snapshot_chapters,
        )?;
    }
    Ok(())
}
