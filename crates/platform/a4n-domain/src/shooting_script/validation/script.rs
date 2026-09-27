use std::collections::HashSet;

use super::ShootingScriptValidationError;
use super::rules::{invalid, positive_index, require_index, required_text};
use super::scene::validate_scene;
use crate::shooting_script::{
    ChapterShootingScriptV2, ShootingDirectingStatementV1, ShootingEditingModeV1,
};

pub(super) fn validate_script(
    script: &ChapterShootingScriptV2,
    pending_units: Option<(usize, usize)>,
) -> Result<(), ShootingScriptValidationError> {
    if let Some(source) = &script.visual_plan_source {
        if source.plan_id <= 0 || source.plan_version <= 0 {
            return invalid("visual_plan_source", "plan_id 和 plan_version 必须大于 0");
        }
        if source.plan_sha256.len() != 64
            || !source
                .plan_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return invalid(
                "visual_plan_source.plan_sha256",
                "必须是 64 位十六进制 SHA-256",
            );
        }
    }
    let mut user_addition_ids = HashSet::new();
    for (offset, addition) in script.user_additions.iter().enumerate() {
        required_text(&addition.id, &format!("user_additions[{offset}].id"))?;
        required_text(
            &addition.description,
            &format!("user_additions[{offset}].description"),
        )?;
        required_text(
            &addition.rationale,
            &format!("user_additions[{offset}].rationale"),
        )?;
        if !user_addition_ids.insert(addition.id.as_str()) {
            return invalid(format!("user_additions[{offset}].id"), "全章必须唯一");
        }
    }
    if let Some(statement) = &script.directing_statement {
        validate_statement(statement)?;
    }
    let mut shot_ids = HashSet::new();
    let mut beat_ids = HashSet::new();
    let mut narrative_units = HashSet::new();
    let mut timeline_positions = HashSet::new();
    for (scene_offset, scene) in script.scenes.iter().enumerate() {
        let scene_path = format!("scenes[{scene_offset}]");
        require_index(
            scene.scene_index,
            scene_offset,
            &format!("{scene_path}.scene_index"),
        )?;
        required_text(
            &scene.scene_identity,
            &format!("{scene_path}.scene_identity"),
        )?;
        positive_index(
            scene.sequence_index,
            &format!("{scene_path}.sequence_index"),
        )?;
        positive_index(
            scene.narrative_unit_index,
            &format!("{scene_path}.narrative_unit_index"),
        )?;
        positive_index(
            scene.timeline_position,
            &format!("{scene_path}.timeline_position"),
        )?;
        if !narrative_units.insert(scene.narrative_unit_index) {
            return invalid(format!("{scene_path}.narrative_unit_index"), "全章必须唯一");
        }
        if !timeline_positions.insert(scene.timeline_position) {
            return invalid(format!("{scene_path}.timeline_position"), "全章必须唯一");
        }
        if matches!(
            scene.editing_mode,
            ShootingEditingModeV1::Intercut | ShootingEditingModeV1::ParallelAction
        ) && scene
            .intercut_group_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return invalid(
                format!("{scene_path}.intercut_group_id"),
                "交叉剪辑或平行行动必须指定分组",
            );
        }
        validate_scene(
            scene,
            &scene_path,
            &mut shot_ids,
            &mut beat_ids,
            &user_addition_ids,
            script.visual_plan_source.is_some(),
        )?;
    }
    for (scene_offset, scene) in script.scenes.iter().enumerate() {
        for (transition_offset, transition) in scene.transition_contracts.iter().enumerate() {
            let target = transition.to_narrative_unit_index as usize;
            if !narrative_units.contains(&transition.to_narrative_unit_index)
                && !pending_units
                    .is_some_and(|(completed, total)| target > completed && target <= total)
            {
                return invalid(
                    format!(
                        "scenes[{scene_offset}].transition_contracts[{transition_offset}].to_narrative_unit_index"
                    ),
                    "必须引用当前章节中的 narrative unit",
                );
            }
            if transition.to_narrative_unit_index == scene.narrative_unit_index {
                return invalid(
                    format!(
                        "scenes[{scene_offset}].transition_contracts[{transition_offset}].to_narrative_unit_index"
                    ),
                    "不能指向当前 narrative unit 自身",
                );
            }
        }
    }
    let expected_positions = (1..=script.scenes.len())
        .map(|position| {
            u32::try_from(position).map_err(|_| ShootingScriptValidationError {
                path: "scenes".into(),
                message: "场景数量超出 u32 范围".into(),
            })
        })
        .collect::<Result<HashSet<_>, _>>()?;
    if timeline_positions != expected_positions {
        return invalid("scenes[].timeline_position", "必须从 1 连续覆盖全部场景");
    }
    Ok(())
}

pub(super) fn validate_statement(
    value: &ShootingDirectingStatementV1,
) -> Result<(), ShootingScriptValidationError> {
    for (field, text) in [
        (
            "central_dramatic_question",
            &value.central_dramatic_question,
        ),
        ("protagonist_throughline", &value.protagonist_throughline),
        ("audience_experience", &value.audience_experience),
        ("viewpoint_strategy", &value.viewpoint_strategy),
        ("visual_strategy", &value.visual_strategy),
        ("spatial_strategy", &value.spatial_strategy),
        ("sound_strategy", &value.sound_strategy),
        ("rhythm_strategy", &value.rhythm_strategy),
    ] {
        required_text(text, &format!("directing_statement.{field}"))?;
    }
    Ok(())
}
