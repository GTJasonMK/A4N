use std::collections::{BTreeMap, BTreeSet};

use super::{
    ChapterVisualPlanSchemaVersion, ChapterVisualPlanV2, ChapterVisualPlanValidationError,
    VisualCameraStateV1, VisualEntityEventV2, VisualEntityStateV1, VisualRelationKindV1,
    VisualSceneV2, VisualShotV2,
};
use crate::shooting_script::ShootingEditingModeV1;

impl ChapterVisualPlanV2 {
    /// 草稿校验允许空场景和未完成的关键帧，保证用户可以先保存再逐步编辑。
    pub fn validate_draft(&self) -> Result<(), ChapterVisualPlanValidationError> {
        validate_plan(self, false)
    }

    /// 确认前的严格校验，要求完整覆盖可执行的场景、镜头和关键帧。
    pub fn validate_confirmed(&self) -> Result<(), ChapterVisualPlanValidationError> {
        validate_plan(self, true)
    }

    pub fn canonical_json(&self) -> Result<serde_json::Value, ChapterVisualPlanValidationError> {
        self.validate_draft()?;
        serde_json::to_value(self).map_err(|error| ChapterVisualPlanValidationError {
            path: "$".into(),
            message: format!("序列化失败: {error}"),
        })
    }
}

fn validate_plan(
    plan: &ChapterVisualPlanV2,
    require_complete: bool,
) -> Result<(), ChapterVisualPlanValidationError> {
    if plan.schema_version != ChapterVisualPlanSchemaVersion::V2 {
        return invalid("schema_version", "必须是 chapter_visual_plan_v2");
    }
    if plan.frame.width == 0 || plan.frame.height == 0 {
        return invalid("frame", "画幅宽高必须大于 0");
    }
    if plan
        .frame
        .background
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return invalid("frame.background", "存在时不能为空");
    }

    let entity_ids = unique_entity_ids(plan)?;
    let mut identity_ids = entity_ids.clone();
    let mut scene_end_ms = 0_u64;
    let mut grouped_scenes = BTreeMap::<&str, Vec<(usize, ShootingEditingModeV1)>>::new();

    if require_complete && plan.scenes.is_empty() {
        return invalid("scenes", "至少包含一个场景");
    }

    for (scene_index, scene) in plan.scenes.iter().enumerate() {
        let scene_path = format!("scenes[{scene_index}]");
        unique_text(&scene.id, &format!("{scene_path}.id"), &mut identity_ids)?;
        required_text(&scene.title, &format!("{scene_path}.title"))?;
        required_text(
            &scene.narrative_purpose,
            &format!("{scene_path}.narrative_purpose"),
        )?;
        validate_refs(
            &scene.participant_entity_ids,
            &entity_ids,
            &format!("{scene_path}.participant_entity_ids"),
        )?;
        if let Some(focal) = scene.focal_entity_id.as_deref() {
            require_known(focal, &entity_ids, &format!("{scene_path}.focal_entity_id"))?;
            if !scene.participant_entity_ids.iter().any(|id| id == focal) {
                return invalid(
                    format!("{scene_path}.focal_entity_id"),
                    "必须属于 participant_entity_ids",
                );
            }
        }
        if let Some(group_id) = scene.intercut_group_id.as_deref() {
            required_text(group_id, &format!("{scene_path}.intercut_group_id"))?;
            if !matches!(
                scene.editing_mode,
                ShootingEditingModeV1::Intercut | ShootingEditingModeV1::ParallelAction
            ) {
                return invalid(
                    format!("{scene_path}.intercut_group_id"),
                    "只有 intercut 或 parallel_action 可以设置交叉剪辑组",
                );
            }
            grouped_scenes
                .entry(group_id)
                .or_default()
                .push((scene_index, scene.editing_mode));
        } else if matches!(
            scene.editing_mode,
            ShootingEditingModeV1::Intercut | ShootingEditingModeV1::ParallelAction
        ) {
            return invalid(
                format!("{scene_path}.intercut_group_id"),
                "intercut 或 parallel_action 必须设置交叉剪辑组",
            );
        }
        if require_complete && scene.shots.is_empty() {
            return invalid(format!("{scene_path}.shots"), "至少包含一个镜头");
        }
        if !scene.shots.is_empty() {
            let scene_duration = scene
                .shots
                .iter()
                .try_fold(0_u64, |total, shot| total.checked_add(shot.duration_ms))
                .ok_or_else(|| error(format!("{scene_path}.shots"), "场景时长超出 u64 范围"))?;
            scene_end_ms = scene_end_ms
                .checked_add(scene_duration)
                .ok_or_else(|| error("scenes", "方案总时长超出 u64 范围"))?;
        }
        validate_shots(
            scene,
            scene_index,
            &entity_ids,
            require_complete,
            &mut identity_ids,
        )?;
    }

    for (group_id, scenes) in grouped_scenes {
        if scenes.len() < 2 {
            return invalid(
                format!("scenes[{}].intercut_group_id", scenes[0].0),
                format!("交叉剪辑组 {group_id} 至少需要两个场景"),
            );
        }
        if scenes.iter().any(|(_, mode)| *mode != scenes[0].1) {
            return invalid(
                format!("scenes[{}].editing_mode", scenes[1].0),
                format!("交叉剪辑组 {group_id} 内的 editing_mode 必须一致"),
            );
        }
    }

    if require_complete && scene_end_ms == 0 {
        return invalid("scenes", "方案总时长必须大于 0");
    }
    Ok(())
}

fn unique_entity_ids(
    plan: &ChapterVisualPlanV2,
) -> Result<BTreeSet<String>, ChapterVisualPlanValidationError> {
    let mut ids = BTreeSet::new();
    for (index, entity) in plan.entities.iter().enumerate() {
        let path = format!("entities[{index}]");
        unique_text(&entity.id, &format!("{path}.id"), &mut ids)?;
        required_text(&entity.label, &format!("{path}.label"))?;
        required_text(&entity.description, &format!("{path}.description"))?;
    }
    Ok(ids)
}

#[allow(clippy::too_many_arguments)]
fn validate_shots(
    scene: &VisualSceneV2,
    scene_index: usize,
    entity_ids: &BTreeSet<String>,
    require_complete: bool,
    identity_ids: &mut BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    for (shot_index, shot) in scene.shots.iter().enumerate() {
        let path = format!("scenes[{scene_index}].shots[{shot_index}]");
        unique_text(&shot.id, &format!("{path}.id"), identity_ids)?;
        required_text(&shot.title, &format!("{path}.title"))?;
        required_text(
            &shot.narrative_purpose,
            &format!("{path}.narrative_purpose"),
        )?;
        if shot.duration_ms == 0 {
            return invalid(format!("{path}.duration_ms"), "必须大于 0");
        }
        validate_keyframes(shot, &path, entity_ids, require_complete, identity_ids)?;
        validate_events(shot, &path, entity_ids, identity_ids)?;
        if let Some(transition) = shot.transition_to_next.as_ref() {
            if shot_index + 1 == scene.shots.len() {
                return invalid(
                    format!("{path}.transition_to_next"),
                    "最后一个镜头不能携带转场",
                );
            }
            if transition.kind == crate::shooting_script::ShootingTransitionTypeV1::Continuous
                && transition.duration_ms != 0
            {
                return invalid(
                    format!("{path}.transition_to_next.duration_ms"),
                    "continuous 转场不应有额外时长",
                );
            }
            if transition.duration_ms > shot.duration_ms
                || transition.duration_ms > scene.shots[shot_index + 1].duration_ms
            {
                return invalid(
                    format!("{path}.transition_to_next.duration_ms"),
                    "转场时长不能超过相邻镜头时长",
                );
            }
        }
    }
    Ok(())
}

fn validate_keyframes(
    shot: &VisualShotV2,
    path: &str,
    entity_ids: &BTreeSet<String>,
    require_complete: bool,
    identity_ids: &mut BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    if require_complete && shot.keyframes.len() < 2 {
        return invalid(format!("{path}.keyframes"), "至少需要首尾两个关键帧");
    }
    let mut cursor_ms = 0_u64;
    for (index, keyframe) in shot.keyframes.iter().enumerate() {
        let keyframe_path = format!("{path}.keyframes[{index}]");
        unique_text(&keyframe.id, &format!("{keyframe_path}.id"), identity_ids)?;
        required_text(&keyframe.label, &format!("{keyframe_path}.label"))?;
        if keyframe.at_ms > shot.duration_ms || keyframe.at_ms < cursor_ms {
            return invalid(
                format!("{keyframe_path}.at_ms"),
                "必须在镜头相对时间范围内并按升序排列",
            );
        }
        cursor_ms = keyframe.at_ms;
        let mut entity_states = BTreeSet::new();
        let mut visible_entities = BTreeSet::new();
        for state in &keyframe.entity_states {
            validate_entity_state(state, entity_ids, &keyframe_path)?;
            if !entity_states.insert(state.entity_id.clone()) {
                return invalid(
                    format!("{keyframe_path}.entity_states[].entity_id"),
                    "同一关键帧不能重复定义实体状态",
                );
            }
            if state.visible {
                visible_entities.insert(state.entity_id.clone());
            }
        }
        validate_camera_state(
            &keyframe.camera,
            entity_ids,
            &visible_entities,
            &keyframe_path,
        )?;
        for relation in &keyframe.relations {
            unique_text(
                &relation.id,
                &format!("{keyframe_path}.relations[].id"),
                identity_ids,
            )?;
            if relation.source_entity_id == relation.target_entity_id {
                return invalid(
                    format!("{keyframe_path}.relations[].target_entity_id"),
                    "不能与源实体相同",
                );
            }
            if !visible_entities.contains(&relation.source_entity_id)
                || !visible_entities.contains(&relation.target_entity_id)
            {
                return invalid(
                    format!("{keyframe_path}.relations"),
                    "关系两端必须在当前关键帧定义为可见实体",
                );
            }
            if relation.kind == VisualRelationKindV1::Custom
                && relation
                    .label
                    .as_deref()
                    .is_none_or(|label| label.trim().is_empty())
            {
                return invalid(
                    format!("{keyframe_path}.relations[].label"),
                    "自定义关系必须填写说明",
                );
            }
        }
    }
    if require_complete
        && (shot.keyframes.first().is_none_or(|value| value.at_ms != 0)
            || shot
                .keyframes
                .last()
                .is_none_or(|value| value.at_ms != shot.duration_ms))
    {
        return invalid(
            format!("{path}.keyframes"),
            "首个和末个关键帧必须锁定镜头边界",
        );
    }
    Ok(())
}

fn validate_events(
    shot: &VisualShotV2,
    path: &str,
    entity_ids: &BTreeSet<String>,
    identity_ids: &mut BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    for (index, event) in shot.entity_events.iter().enumerate() {
        let event_path = format!("{path}.entity_events[{index}]");
        validate_event_identity(
            event,
            &event_path,
            shot.duration_ms,
            entity_ids,
            identity_ids,
        )?;
    }
    for (index, event) in shot.camera_events.iter().enumerate() {
        let event_path = format!("{path}.camera_events[{index}]");
        if event.start_ms >= event.end_ms || event.end_ms > shot.duration_ms {
            return invalid(format!("{event_path}.time"), "必须位于镜头相对时间范围内");
        }
        unique_text(&event.id, &format!("{event_path}.id"), identity_ids)?;
        required_text(&event.description, &format!("{event_path}.description"))?;
        if let Some(target) = event.target_entity_id.as_deref() {
            require_known(
                target,
                entity_ids,
                &format!("{event_path}.target_entity_id"),
            )?;
        }
    }
    for (index, event) in shot.sound_events.iter().enumerate() {
        let event_path = format!("{path}.sound_events[{index}]");
        if event.start_ms >= event.end_ms || event.end_ms > shot.duration_ms {
            return invalid(format!("{event_path}.time"), "必须位于镜头相对时间范围内");
        }
        unique_text(&event.id, &format!("{event_path}.id"), identity_ids)?;
        required_text(&event.content, &format!("{event_path}.content"))?;
        if let Some(speaker) = event.speaker_entity_id.as_deref() {
            require_known(
                speaker,
                entity_ids,
                &format!("{event_path}.speaker_entity_id"),
            )?;
        }
    }
    for (index, event) in shot.technique_events.iter().enumerate() {
        let event_path = format!("{path}.technique_events[{index}]");
        if event.start_ms >= event.end_ms || event.end_ms > shot.duration_ms {
            return invalid(format!("{event_path}.time"), "必须位于镜头相对时间范围内");
        }
        unique_text(&event.id, &format!("{event_path}.id"), identity_ids)?;
        required_text(&event.description, &format!("{event_path}.description"))?;
    }
    Ok(())
}

fn validate_event_identity(
    event: &VisualEntityEventV2,
    path: &str,
    shot_duration_ms: u64,
    entity_ids: &BTreeSet<String>,
    identity_ids: &mut BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    if event.start_ms >= event.end_ms || event.end_ms > shot_duration_ms {
        return invalid(
            format!("{path}.time"),
            "必须位于镜头相对时间范围内且满足 start < end",
        );
    }
    unique_text(&event.id, &format!("{path}.id"), identity_ids)?;
    required_text(&event.description, &format!("{path}.description"))?;
    require_known(&event.entity_id, entity_ids, &format!("{path}.entity_id"))?;
    if let Some(target) = event.target_entity_id.as_deref() {
        require_known(target, entity_ids, &format!("{path}.target_entity_id"))?;
    }
    Ok(())
}

fn validate_camera_state(
    camera: &VisualCameraStateV1,
    entity_ids: &BTreeSet<String>,
    keyframe_entity_ids: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    if !camera.center_x.is_finite()
        || !camera.center_y.is_finite()
        || !camera.rotation_degrees.is_finite()
        || !camera.zoom.is_finite()
        || !(0.0..=1.0).contains(&camera.center_x)
        || !(0.0..=1.0).contains(&camera.center_y)
        || camera.zoom <= 0.0
    {
        return invalid(
            format!("{path}.camera"),
            "相机数值必须有限，中心点范围为 0..=1，zoom 必须大于 0",
        );
    }
    if let Some(focus) = camera.focus_entity_id.as_deref() {
        require_known(focus, entity_ids, &format!("{path}.camera.focus_entity_id"))?;
        if !keyframe_entity_ids.contains(focus) {
            return invalid(
                format!("{path}.camera.focus_entity_id"),
                "焦点实体必须在当前关键帧中定义状态",
            );
        }
    }
    Ok(())
}

fn validate_entity_state(
    state: &VisualEntityStateV1,
    entity_ids: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    require_known(
        &state.entity_id,
        entity_ids,
        &format!("{path}.entity_states[].entity_id"),
    )?;
    let bounds = &state.bounds;
    if [
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        state.rotation_degrees,
        state.opacity,
    ]
    .into_iter()
    .any(|value| !value.is_finite())
        || bounds.x < 0.0
        || bounds.y < 0.0
        || bounds.width <= 0.0
        || bounds.height <= 0.0
        || bounds.x + bounds.width > 1.0
        || bounds.y + bounds.height > 1.0
        || !(0.0..=1.0).contains(&state.opacity)
    {
        return invalid(
            format!("{path}.entity_states"),
            "位置、尺寸、旋转和透明度必须是有限值，透明度范围为 0..=1",
        );
    }
    if let Some(target) = state.gaze_target_entity_id.as_deref() {
        require_known(
            target,
            entity_ids,
            &format!("{path}.entity_states[].gaze_target_entity_id"),
        )?;
    }
    Ok(())
}

fn validate_refs(
    values: &[String],
    known: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    let mut seen = BTreeSet::new();
    for value in values {
        required_text(value, path)?;
        if !seen.insert(value) {
            return invalid(path, "不能重复引用同一个 ID");
        }
        require_known(value, known, path)?;
    }
    Ok(())
}

fn unique_text(
    value: &str,
    path: &str,
    ids: &mut BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    required_text(value, path)?;
    if !ids.insert(value.to_string()) {
        invalid(path, "必须唯一")
    } else {
        Ok(())
    }
}

fn required_text(value: &str, path: &str) -> Result<(), ChapterVisualPlanValidationError> {
    if value.trim().is_empty() {
        invalid(path, "不能为空")
    } else {
        Ok(())
    }
}

fn require_known(
    value: &str,
    ids: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    if ids.contains(value) {
        Ok(())
    } else {
        invalid(path, "引用了不存在的 ID")
    }
}

fn error(path: impl Into<String>, message: impl Into<String>) -> ChapterVisualPlanValidationError {
    ChapterVisualPlanValidationError {
        path: path.into(),
        message: message.into(),
    }
}

fn invalid(
    path: impl Into<String>,
    message: impl Into<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    Err(error(path, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_visual_plan::{
        ChapterVisualPlanV2, VisualCameraStateV1, VisualEntityEventKindV1, VisualEntityEventV2,
        VisualEntityKindV1, VisualEntityStateV1, VisualEntityV2, VisualFrameV2,
        VisualInterpolationV1, VisualKeyframeV2, VisualRelationKindV1, VisualRelationV1,
        VisualSceneV2, VisualShotV2,
    };
    use crate::shooting_script::{ShootingCoverageV1, ShootingEditingModeV1};

    fn shot(id: &str, duration_ms: u64) -> VisualShotV2 {
        VisualShotV2 {
            id: id.into(),
            title: id.into(),
            duration_ms,
            narrative_purpose: "推进情节".into(),
            coverage: ShootingCoverageV1::MustShow,
            shot_size: None,
            camera_angle: None,
            camera_height: None,
            lens_intent: None,
            composition: None,
            keyframes: vec![],
            entity_events: vec![],
            camera_events: vec![],
            sound_events: vec![],
            technique_events: vec![],
            continuity_notes: vec![],
            transition_to_next: None,
        }
    }

    fn plan() -> ChapterVisualPlanV2 {
        ChapterVisualPlanV2 {
            schema_version: ChapterVisualPlanSchemaVersion::V2,
            frame: VisualFrameV2 {
                width: 1920,
                height: 1080,
                background: None,
            },
            entities: vec![],
            scenes: vec![VisualSceneV2 {
                id: "scene-1".into(),
                title: "场景".into(),
                narrative_purpose: "推进".into(),
                editing_mode: ShootingEditingModeV1::Continuity,
                intercut_group_id: None,
                location: None,
                time_of_day: None,
                participant_entity_ids: vec![],
                focal_entity_id: None,
                shots: vec![shot("shot-1", 1_000)],
            }],
        }
    }

    fn entity(id: &str) -> VisualEntityV2 {
        VisualEntityV2 {
            id: id.into(),
            label: id.into(),
            kind: VisualEntityKindV1::Character,
            description: id.into(),
        }
    }

    fn entity_state(id: &str, visible: bool) -> VisualEntityStateV1 {
        VisualEntityStateV1 {
            entity_id: id.into(),
            visible,
            bounds: super::super::VisualRectV1 {
                x: 0.1,
                y: 0.1,
                width: 0.2,
                height: 0.2,
            },
            rotation_degrees: 0.0,
            opacity: 1.0,
            z_index: 0,
            facing: super::super::VisualFacingV1::Front,
            expression: None,
            pose: None,
            action: None,
            gaze_target_entity_id: None,
            dialogue: None,
            notes: None,
        }
    }

    fn keyframe(
        id: &str,
        at_ms: u64,
        entity_states: Vec<VisualEntityStateV1>,
        relations: Vec<VisualRelationV1>,
    ) -> VisualKeyframeV2 {
        VisualKeyframeV2 {
            id: id.into(),
            at_ms,
            label: id.into(),
            interpolation_to_next: VisualInterpolationV1::Hold,
            camera: VisualCameraStateV1 {
                center_x: 0.5,
                center_y: 0.5,
                zoom: 1.0,
                rotation_degrees: 0.0,
                focus_entity_id: None,
                framing_note: None,
            },
            entity_states,
            relations,
        }
    }

    #[test]
    fn draft_can_be_empty_but_confirmed_plan_cannot() {
        let mut value = plan();
        value.scenes.clear();
        value.validate_draft().expect("empty draft");
        assert!(value.validate_confirmed().is_err());
    }

    #[test]
    fn all_visual_identity_ids_are_unique_across_kinds() {
        let mut value = plan();
        value.entities = vec![entity("scene-1")];
        let error = value
            .validate_draft()
            .expect_err("entity and scene IDs must share one namespace");
        assert_eq!(error.path, "scenes[0].id");
    }

    #[test]
    fn confirmed_plan_requires_keyframe_boundaries() {
        let error = plan()
            .validate_confirmed()
            .expect_err("keyframe boundaries are required");
        assert!(error.path.ends_with("keyframes"));
    }

    #[test]
    fn relative_events_must_stay_inside_the_shot() {
        let mut value = plan();
        value.scenes[0].shots[0]
            .camera_events
            .push(super::super::VisualCameraEventV2 {
                id: "camera-1".into(),
                kind: super::super::VisualCameraEventKindV1::Pan,
                start_ms: 500,
                end_ms: 1_001,
                target_entity_id: None,
                description: "推镜".into(),
                parameters: Default::default(),
            });
        let error = value.validate_draft().expect_err("event exceeds shot");
        assert!(error.path.ends_with("time"));
    }

    #[test]
    fn entity_events_must_end_inside_the_shot() {
        let mut value = plan();
        value.entities = vec![entity("hero")];
        value.scenes[0].shots[0]
            .entity_events
            .push(VisualEntityEventV2 {
                id: "entity-event-1".into(),
                kind: VisualEntityEventKindV1::Move,
                start_ms: 500,
                end_ms: 1_001,
                entity_id: "hero".into(),
                target_entity_id: None,
                description: "移动".into(),
                parameters: Default::default(),
            });
        let error = value.validate_draft().expect_err("event exceeds shot");
        assert!(error.path.ends_with("time"));
    }

    #[test]
    fn relations_require_visible_entity_states() {
        let mut value = plan();
        value.entities = vec![entity("hidden"), entity("visible")];
        let relation = VisualRelationV1 {
            id: "relation-1".into(),
            kind: VisualRelationKindV1::LeftOf,
            source_entity_id: "hidden".into(),
            target_entity_id: "visible".into(),
            label: None,
        };
        value.scenes[0].shots[0].keyframes = vec![
            keyframe(
                "keyframe-start",
                0,
                vec![entity_state("hidden", false), entity_state("visible", true)],
                vec![relation],
            ),
            keyframe(
                "keyframe-end",
                1_000,
                vec![entity_state("hidden", false), entity_state("visible", true)],
                vec![],
            ),
        ];
        let error = value
            .validate_draft()
            .expect_err("hidden relation endpoint");
        assert!(error.message.contains("可见实体"));
    }

    #[test]
    fn intercut_groups_are_limited_to_intercut_modes() {
        let mut value = plan();
        value.scenes[0].intercut_group_id = Some("group-1".into());
        let error = value
            .validate_draft()
            .expect_err("continuity scene cannot use an intercut group");
        assert!(error.path.ends_with("intercut_group_id"));
        assert!(error.message.contains("只有 intercut"));
    }

    #[test]
    fn intercut_groups_need_two_scenes_with_one_mode() {
        let mut value = plan();
        value.scenes[0].editing_mode = ShootingEditingModeV1::Intercut;
        value.scenes[0].intercut_group_id = Some("group-1".into());
        let error = value
            .validate_draft()
            .expect_err("an intercut group needs two scenes");
        assert!(error.message.contains("至少需要两个场景"));

        let mut second = value.scenes[0].clone();
        second.id = "scene-2".into();
        for shot in &mut second.shots {
            shot.id = "shot-2".into();
            for keyframe in &mut shot.keyframes {
                keyframe.id = format!("{}-2", keyframe.id);
            }
        }
        value.scenes.push(second);
        value.validate_draft().expect("matching intercut group");

        value.scenes[1].editing_mode = ShootingEditingModeV1::ParallelAction;
        let error = value
            .validate_draft()
            .expect_err("intercut group modes must match");
        assert!(error.message.contains("editing_mode 必须一致"));
    }
}
