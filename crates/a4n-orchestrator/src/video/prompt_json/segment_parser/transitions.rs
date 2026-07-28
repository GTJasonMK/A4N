use std::collections::HashSet;

use a4n_core::{
    error::AppError, shooting_script::ShootingTransitionTypeV1,
    video_execution::VideoExecutionShotV2,
};

use super::{VideoSegmentPlan, VideoShootingScriptProjection};

pub(super) fn apply_segment_transitions(
    segments: &mut [VideoSegmentPlan],
    projection: &VideoShootingScriptProjection,
) -> Result<(), AppError> {
    use a4n_core::video_execution::VideoIncomingTransitionV1;

    if segments.len() < 2 {
        return Ok(());
    }
    for index in 0..segments.len() - 1 {
        let from_shot_id = segments[index].source_shot_id.clone();
        let to_shot_id = segments[index + 1].source_shot_id.clone();
        if from_shot_id == to_shot_id {
            continue;
        }
        let from_shot = projection.shot(&from_shot_id).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划引用不存在的 source_shot_id {from_shot_id}"
            ))
        })?;
        let to_shot = projection.shot(&to_shot_id).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划引用不存在的 source_shot_id {to_shot_id}"
            ))
        })?;
        let contract = from_shot
            .transition_contracts
            .iter()
            .find(|contract| contract.to_narrative_unit_index == to_shot.narrative_unit_index);
        if from_shot.narrative_unit_index != to_shot.narrative_unit_index && contract.is_none() {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划镜头 {from_shot_id} → {to_shot_id} 跨叙事单元，但拍摄剧本缺少精确 transition_contract"
            )));
        }

        let transition_type = ShootingTransitionTypeV1::resolve_boundary(
            &from_shot.transition_out,
            &to_shot.transition_in,
            contract.map(|value| value.transition_type),
        )
        .map_err(|error| {
            AppError::InvalidArgument(error.shot_boundary_message(
                &from_shot_id,
                &to_shot_id,
                &from_shot.transition_out,
                &to_shot.transition_in,
            ))
        })?;
        let continuity_to_preserve = unique_transition_strings(
            from_shot
                .continuity_requirements
                .iter()
                .chain(to_shot.continuity_requirements.iter()),
        );
        let audience_knowledge_to_preserve = unique_transition_strings(
            contract
                .into_iter()
                .flat_map(|value| value.preserved_knowledge.iter()),
        );
        let discontinuity_to_establish = transition_discontinuities(projection, from_shot, to_shot);
        segments[index + 1].incoming_transition = Some(VideoIncomingTransitionV1 {
            source_shot_id: from_shot_id,
            target_shot_id: to_shot_id,
            transition_type,
            editing_mode: from_shot.editing_mode,
            source_transition_out: from_shot.transition_out.clone(),
            target_transition_in: to_shot.transition_in.clone(),
            preserved_action: contract.and_then(|value| value.preserved_action.clone()),
            preserved_sound: contract.and_then(|value| value.preserved_sound.clone()),
            required_cut_point: contract.and_then(|value| value.required_cut_point.clone()),
            continuity_to_preserve,
            audience_knowledge_to_preserve,
            discontinuity_to_establish,
        });
    }
    Ok(())
}

fn transition_discontinuities(
    projection: &VideoShootingScriptProjection,
    from_shot: &VideoExecutionShotV2,
    to_shot: &VideoExecutionShotV2,
) -> Vec<String> {
    let mut values = Vec::new();
    if from_shot.narrative_unit_index != to_shot.narrative_unit_index {
        values.push(format!(
            "叙事单元由 {} 切换为 {}",
            from_shot.narrative_unit_index, to_shot.narrative_unit_index
        ));
    }
    if from_shot.timeline_position != to_shot.timeline_position {
        values.push(format!(
            "时间线位置由 {} 切换为 {}",
            from_shot.timeline_position, to_shot.timeline_position
        ));
    }
    let from_scene = projection.scenes.iter().find(|scene| {
        scene
            .shots
            .iter()
            .any(|shot| shot.source_shot_id == from_shot.source_shot_id)
    });
    let to_scene = projection.scenes.iter().find(|scene| {
        scene
            .shots
            .iter()
            .any(|shot| shot.source_shot_id == to_shot.source_shot_id)
    });
    if let (Some(from_scene), Some(to_scene)) = (from_scene, to_scene)
        && from_scene.scene_identity != to_scene.scene_identity
    {
        values.push(format!(
            "场景由“{}”切换为“{}”",
            from_scene.scene_identity, to_scene.scene_identity
        ));
    }
    values
}

fn unique_transition_strings<'a>(values: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert((*value).to_string()))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod transition_boundary_tests {
    use super::*;

    fn resolve_as_app_error(
        from_shot_id: &str,
        to_shot_id: &str,
        transition_out: &str,
        transition_in: &str,
        contract_type: Option<ShootingTransitionTypeV1>,
    ) -> Result<ShootingTransitionTypeV1, AppError> {
        ShootingTransitionTypeV1::resolve_boundary(transition_out, transition_in, contract_type)
            .map_err(|error| {
                AppError::InvalidArgument(error.shot_boundary_message(
                    from_shot_id,
                    to_shot_id,
                    transition_out,
                    transition_in,
                ))
            })
    }

    #[test]
    fn shot_level_hard_cut_is_normalized_without_scene_contract() {
        let kind = resolve_as_app_error(
            "unit_1_shot_1",
            "unit_1_shot_2",
            "hard_cut：人物骤然抬眼时切出",
            "hard_cut：切入洗漱镜前正面中景",
            None,
        )
        .expect("shot-level hard cut");

        assert_eq!(kind, ShootingTransitionTypeV1::HardCut);
    }

    #[test]
    fn missing_transition_method_is_rejected_instead_of_defaulting_to_hard_cut() {
        let err = resolve_as_app_error(
            "unit_1_shot_1",
            "unit_1_shot_2",
            "人物停在门前",
            "新镜头建立门内构图",
            None,
        )
        .expect_err("missing transition method must fail");

        assert!(err.to_string().contains("transition_out 缺少规范转场值"));
    }

    #[test]
    fn scene_contract_and_shot_transition_must_not_conflict() {
        let err = resolve_as_app_error(
            "unit_1_shot_1",
            "unit_2_shot_1",
            "whip_pan：向右甩镜切出",
            "whip_pan：向右甩镜落幅",
            Some(ShootingTransitionTypeV1::HardCut),
        )
        .expect_err("conflicting transition sources must fail");

        assert!(
            err.to_string()
                .contains("场景转场 hard_cut 与镜头转场 whip_pan 冲突")
        );
    }

    #[test]
    fn transition_out_and_in_must_name_the_same_method() {
        let err = resolve_as_app_error(
            "unit_1_shot_1",
            "unit_1_shot_2",
            "hard_cut：在人物抬眼时切出",
            "dissolve：叠化进入洗漱台",
            None,
        )
        .expect_err("boundary sides must not disagree");

        assert!(err.to_string().contains("transition_out=hard_cut"));
        assert!(err.to_string().contains("transition_in=dissolve"));
    }

    #[test]
    fn sound_bridge_contract_can_accompany_visual_hard_cut() {
        let kind = resolve_as_app_error(
            "unit_1_shot_1",
            "unit_2_shot_1",
            "hard_cut：闹钟声延续到切点",
            "hard_cut：切入洗漱镜前",
            Some(ShootingTransitionTypeV1::SoundBridge),
        )
        .expect("sound bridge with hard cut");

        assert_eq!(kind, ShootingTransitionTypeV1::HardCut);
    }

    #[test]
    fn ambiguous_transition_methods_are_rejected() {
        let err = resolve_as_app_error(
            "unit_1_shot_1",
            "unit_1_shot_2",
            "hard_cut 或 dissolve",
            "hard_cut：进入洗漱台",
            None,
        )
        .expect_err("ambiguous transition must fail");

        assert!(err.to_string().contains("必须只指定一种确定手法"));
    }
}
