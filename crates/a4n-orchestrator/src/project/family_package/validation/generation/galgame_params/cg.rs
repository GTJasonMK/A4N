use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    project::source_snapshot_profile,
};
use std::collections::HashMap;

use crate::project::{
    generation_spec_params_object, optional_params_i64, required_generation_spec_target_id,
    required_params_i64, required_params_object, required_params_string,
    validate_generation_spec_canonical_key, validate_optional_galgame_cg_importance,
    validate_params_id_equals, validate_snapshot_character_profile_id,
};

use super::super::targets::validate_generation_spec_galgame_scene_context;

pub(super) fn validate_galgame_cg_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    let target_id = required_generation_spec_target_id(spec)?;
    let target_line = galgame_lines.get(&target_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame CG 参数目标行不存在: generation_spec_id={}, target_entity_id={target_id}",
            spec.id
        )
    })?;
    let params = generation_spec_params_object(spec)?;
    let expected_key = format!("galgame:{}:cg:line:{target_id}", spec.project_id);
    validate_generation_spec_canonical_key(spec, params, &expected_key)?;
    let line = required_params_object(params, "line", spec.id)?;
    validate_params_id_equals(spec, line, "line.id", target_id)?;
    if let Some(profile_id) = optional_params_i64(line, "line.speaker_profile_id", spec.id)? {
        validate_snapshot_character_profile_id(
            spec,
            source_snapshot_id,
            profile_id,
            snapshot_profiles,
        )?;
    }
    let scene = required_params_object(params, "scene", spec.id)?;
    let scene_id = required_params_i64(scene, "scene.id", spec.id)?;
    let target_scene = galgame_scenes.get(&scene_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame CG 参数 scene.id 不存在: generation_spec_id={}, scene_id={scene_id}",
            spec.id
        )
    })?;
    ensure_package!(
        target_line.scene_id == scene_id,
        "项目族导入包 Galgame CG 参数 scene.id 与目标行所属场不一致: generation_spec_id={}, line_id={}, params_scene_id={}, actual_scene_id={}",
        spec.id,
        target_id,
        scene_id,
        target_line.scene_id
    );
    validate_generation_spec_galgame_scene_context(
        spec,
        source_snapshot_id,
        target_scene,
        galgame_scripts,
    )?;
    required_params_string(params, "cg_note", spec.id)?;
    validate_optional_galgame_cg_importance(params, spec.id)?;
    Ok(())
}
