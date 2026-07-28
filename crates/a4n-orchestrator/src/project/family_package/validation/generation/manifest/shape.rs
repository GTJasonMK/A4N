use crate::generation::{
    is_valid_generation_entity_kind, is_valid_generation_entity_role,
    is_valid_generation_media_ref_role, is_valid_generation_prompt_kind,
    is_valid_generation_spec_status, is_valid_generation_spec_target_entity_kind,
    is_valid_generation_spec_target_kind, is_valid_generation_text_anchor_role,
};

use crate::project::family_package::ProjectFamilyPackage;

pub(super) fn validate_generation_shape_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    for spec in &package.generation_specs {
        ensure_package!(
            is_valid_generation_spec_target_kind(&spec.target_kind),
            "项目族导入包生成意图目标类型无效: generation_spec_id={}, target_kind={}",
            spec.id,
            spec.target_kind
        );
        ensure_package!(
            is_valid_generation_spec_target_entity_kind(&spec.target_entity_kind),
            "项目族导入包生成意图目标实体类型无效: generation_spec_id={}, target_entity_kind={}",
            spec.id,
            spec.target_entity_kind
        );
        ensure_package!(
            is_valid_generation_spec_status(&spec.status),
            "项目族导入包生成意图状态无效: generation_spec_id={}, status={}",
            spec.id,
            spec.status
        );
        ensure_package!(
            spec.params_json.is_object(),
            "项目族导入包生成意图参数必须是 JSON 对象: generation_spec_id={}",
            spec.id
        );
    }

    for record in &package.generation_spec_text_anchors {
        ensure_package!(
            is_valid_generation_text_anchor_role(&record.role),
            "项目族导入包生成意图文本锚点角色无效: generation_spec_id={}, text_anchor_id={}, role={}",
            record.generation_spec_id,
            record.text_anchor_id,
            record.role
        );
    }

    for record in &package.generation_spec_entity_refs {
        ensure_package!(
            is_valid_generation_entity_kind(&record.entity_kind),
            "项目族导入包生成意图实体类型无效: generation_spec_id={}, entity_id={}, entity_kind={}",
            record.generation_spec_id,
            record.entity_id,
            record.entity_kind
        );
        ensure_package!(
            is_valid_generation_entity_role(&record.role),
            "项目族导入包生成意图实体角色无效: generation_spec_id={}, entity_id={}, role={}",
            record.generation_spec_id,
            record.entity_id,
            record.role
        );
    }

    for record in &package.generation_spec_media_refs {
        ensure_package!(
            is_valid_generation_media_ref_role(&record.role),
            "项目族导入包生成意图媒体角色无效: generation_spec_id={}, media_asset_id={}, role={}",
            record.generation_spec_id,
            record.media_asset_id,
            record.role
        );
    }

    for artifact in &package.prompt_artifacts {
        ensure_package!(
            is_valid_generation_prompt_kind(&artifact.prompt_kind),
            "项目族导入包提示词类型无效: prompt_artifact_id={}, prompt_kind={}",
            artifact.id,
            artifact.prompt_kind
        );
    }
    Ok(())
}
