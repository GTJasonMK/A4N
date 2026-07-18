use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    project::source_snapshot_profile,
};
use std::collections::{BTreeSet, HashMap};

use super::super::super::galgame::{
    is_valid_galgame_expression, is_valid_galgame_position, normalized_galgame_position,
};
use super::super::{
    params::{
        generation_spec_params_object, optional_params_string, required_generation_spec_target_id,
        required_params_i64, required_params_i64_array, required_params_string,
        required_params_string_array, validate_generation_spec_canonical_key,
        validate_snapshot_character_profile_id,
    },
    targets::validate_generation_spec_galgame_scene_context,
};

pub(super) fn validate_galgame_portrait_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    let target_id = required_generation_spec_target_id(spec)?;
    ensure_package!(
        spec.source_chapter_id.is_none() && spec.source_snapshot_chapter_id.is_none(),
        "项目族导入包 Galgame 立绘差分生成意图必须使用源快照级上下文: generation_spec_id={}, source_chapter_id={:?}, source_snapshot_chapter_id={:?}",
        spec.id,
        spec.source_chapter_id,
        spec.source_snapshot_chapter_id
    );
    validate_snapshot_character_profile_id(spec, source_snapshot_id, target_id, snapshot_profiles)?;
    let params = generation_spec_params_object(spec)?;
    let character_profile_id = required_params_i64(params, "character_profile_id", spec.id)?;
    ensure_package!(
        character_profile_id == target_id,
        "项目族导入包 Galgame 立绘差分参数 character_profile_id 与 target_entity_id 不一致: generation_spec_id={}, character_profile_id={}, target_entity_id={}",
        spec.id,
        character_profile_id,
        target_id
    );
    let expression = required_params_string(params, "expression", spec.id)?;
    ensure_package!(
        is_valid_galgame_expression(&expression),
        "项目族导入包 Galgame 立绘差分参数 expression 无效: generation_spec_id={}, expression={}",
        spec.id,
        expression
    );
    let pose = optional_params_string(params, "pose", spec.id)?;
    let variant_key = match pose {
        Some(ref pose) => format!("expression:{expression}:pose:{pose}"),
        None => format!("expression:{expression}"),
    };
    let expected_key = format!(
        "galgame:{}:portrait:{target_id}:{variant_key}",
        spec.project_id
    );
    validate_generation_spec_canonical_key(spec, params, &expected_key)?;
    let source_line_ids = required_params_i64_array(params, "source_line_ids", spec.id)?;
    let position_samples = required_params_string_array(params, "position_samples", spec.id)?;
    let mut source_positions = BTreeSet::new();
    for line_id in source_line_ids {
        let line = galgame_lines.get(&line_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 立绘差分参数 source_line_ids 引用了不存在的行: generation_spec_id={}, line_id={line_id}",
                spec.id
            )
        })?;
        ensure_package!(
            line.speaker_profile_id == Some(target_id),
            "项目族导入包 Galgame 立绘差分参数 source_line_ids 行角色与目标不一致: generation_spec_id={}, line_id={}, speaker_profile_id={:?}, expected={}",
            spec.id,
            line.id,
            line.speaker_profile_id,
            target_id
        );
        ensure_package!(
            line.expression.as_deref().map(str::trim) == Some(expression.as_str()),
            "项目族导入包 Galgame 立绘差分参数 source_line_ids 行 expression 与目标不一致: generation_spec_id={}, line_id={}, expression={:?}, expected={}",
            spec.id,
            line.id,
            line.expression,
            expression
        );
        ensure_package!(
            line.pose
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                == pose.as_deref(),
            "项目族导入包 Galgame 立绘差分参数 source_line_ids 行 pose 与目标不一致: generation_spec_id={}, line_id={}, pose={:?}, expected={:?}",
            spec.id,
            line.id,
            line.pose,
            pose
        );
        if let Some(position) = normalized_galgame_position(line) {
            ensure_package!(
                is_valid_galgame_position(position),
                "项目族导入包 Galgame 立绘差分参数 source_line_ids 行 position 无效: generation_spec_id={}, line_id={}, position={}",
                spec.id,
                line.id,
                position
            );
            source_positions.insert(position.to_string());
        }
        let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 立绘差分参数 source_line_ids 行所属场不存在: generation_spec_id={}, line_id={}, scene_id={}",
                spec.id, line_id, line.scene_id
            )
        })?;
        validate_generation_spec_galgame_scene_context(
            spec,
            source_snapshot_id,
            scene,
            galgame_scripts,
        )?;
    }
    validate_galgame_portrait_position_samples(spec.id, &position_samples, &source_positions)?;
    Ok(())
}

fn validate_galgame_portrait_position_samples(
    spec_id: i64,
    position_samples: &[String],
    source_positions: &BTreeSet<String>,
) -> Result<(), String> {
    let mut sample_positions = BTreeSet::new();
    for position in position_samples {
        ensure_package!(
            is_valid_galgame_position(position),
            "项目族导入包 Galgame 立绘差分参数 position_samples 包含无效站位: generation_spec_id={}, position={}",
            spec_id,
            position
        );
        sample_positions.insert(position.clone());
    }
    ensure_package!(
        sample_positions == *source_positions,
        "项目族导入包 Galgame 立绘差分参数 position_samples 与 source_line_ids 行站位不一致: generation_spec_id={}, position_samples={:?}, source_positions={:?}",
        spec_id,
        sample_positions,
        source_positions
    );
    Ok(())
}
