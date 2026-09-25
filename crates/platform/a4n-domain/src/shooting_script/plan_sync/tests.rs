use crate::chapter_visual_plan::{
    ChapterVisualPlanV3, SceneBindingV3, VisualPlanEditOperationV3, VisualPointV3,
    VisualSegmentProjectionV3, VisualSegmentV3,
};
use crate::shooting_script::*;

fn point(id: &str, at_ms: u64) -> VisualPointV3 {
    VisualPointV3 {
        id: id.into(),
        at_ms,
        label: id.into(),
        image_id: None,
        description: format!("{id} 画面"),
        reference_image_ids: Vec::new(),
    }
}

fn segment(id: &str, start: &str, end: &str, scene: i64) -> VisualSegmentV3 {
    VisualSegmentV3 {
        id: id.into(),
        title: String::new(),
        start_point_id: start.into(),
        end_point_id: end.into(),
        prompt: format!("{id} 的人物行动"),
        scene_binding: Some(SceneBindingV3::Scene {
            scene_profile_id: scene,
        }),
        use_end_point_image: false,
    }
}

/// 两个场景、四个镜头：shot-1/shot-2 在场景 1，shot-3/shot-4 在场景 2。
fn plan() -> ChapterVisualPlanV3 {
    let mut plan = ChapterVisualPlanV3::starter();
    plan.points = vec![
        point("start", 0),
        point("p1", 4_000),
        point("p2", 8_000),
        point("p3", 12_000),
        point("end", 16_000),
    ];
    plan.segments = vec![
        segment("shot-1", "start", "p1", 1),
        segment("shot-2", "p1", "p2", 1),
        segment("shot-3", "p2", "p3", 2),
        segment("shot-4", "p3", "end", 2),
    ];
    plan.validate_confirmed().expect("fixture plan");
    plan
}

fn source(version: i32) -> ShootingVisualPlanSourceV1 {
    ShootingVisualPlanSourceV1 {
        plan_id: version.into(),
        plan_version: version,
        plan_sha256: format!("{version:064x}"),
    }
}

fn shot(planned: &VisualSegmentProjectionV3, index: u32) -> ShootingShotV1 {
    let addition = segment_addition_id(&planned.segment_id);
    let seconds = (planned.end_ms - planned.start_ms) as f64 / 1000.0;
    ShootingShotV1 {
        shot_index: index,
        shot_id: planned.segment_id.clone(),
        title: planned.title.clone(),
        source_span_ids: Vec::new(),
        visual_shot_id: Some(planned.segment_id.clone()),
        planned_start_ms: Some(planned.start_ms),
        planned_end_ms: Some(planned.end_ms),
        user_addition_ids: vec![addition.clone()],
        visual_constraints: Some(planned.clone()),
        coverage: ShootingCoverageV1::MustShow,
        narrative_purpose: planned.prompt.clone(),
        subject: "主角".into(),
        shot_size: ShootingShotSizeV1::Medium,
        camera_angle: ShootingCameraAngleV1::EyeLevel,
        camera_height: ShootingCameraHeightV1::Eye,
        lens_intent: format!("{} 镜头意图", planned.segment_id),
        camera_movement: ShootingCameraMovementV1::Locked,
        composition: "主角居中".into(),
        spatial_anchors: Vec::new(),
        blocking: ShootingBlockingV1 {
            start: "门口".into(),
            movement: "前行".into(),
            end: "走廊".into(),
        },
        performance_beats: vec![ShootingPerformanceBeatV1 {
            beat_index: 1,
            beat_id: format!("{}_beat_1", planned.segment_id),
            source_span_ids: Vec::new(),
            user_addition_ids: vec![addition],
            actor: Some("主角".into()),
            objective: "前进".into(),
            obstacle: "昏暗".into(),
            visible_action: "放慢脚步".into(),
            reaction: None,
            expression: None,
            gaze: None,
            pose_or_movement: None,
            dialogue_intent: None,
            audience_question: "前方有什么".into(),
            answer_or_delay: "暂不揭示".into(),
            knowledge_delta: Vec::new(),
            relationship_delta: None,
            spatial_change: None,
            entry_state: "谨慎".into(),
            exit_state: "警惕".into(),
            required: true,
            estimated_seconds: seconds,
        }],
        dialogue_cues: Vec::new(),
        sound: ShootingSoundDesignV1 {
            dialogue: Vec::new(),
            environment: vec!["回声".into()],
            effects: Vec::new(),
            music: Vec::new(),
            silence: Vec::new(),
        },
        camera_start_state: "中景".into(),
        camera_end_state: "中景".into(),
        start_state: "进入".into(),
        end_state: "停下".into(),
        transition_in: "切入".into(),
        transition_out: "切出".into(),
        continuity_requirements: Vec::new(),
        estimated_natural_seconds: seconds,
    }
}

/// 模拟一次完整生成：每个叙事单元一个场景，场景间有指向下一单元的转场合同。
fn script(plan: &ChapterVisualPlanV3) -> ChapterShootingScriptV1 {
    let projected = project_visual_plan_for_shooting_script(plan, &[]).unwrap();
    let unit_count = projected.narrative_units.len() as u32;
    let scenes = projected
        .narrative_units
        .iter()
        .enumerate()
        .map(|(offset, unit)| {
            let index = offset as u32 + 1;
            ShootingSceneV1 {
                scene_index: index,
                scene_identity: unit.id.clone(),
                sequence_index: unit.sequence_index,
                narrative_unit_index: index,
                timeline_position: index,
                intercut_group_id: None,
                editing_mode: ShootingEditingModeV1::Continuity,
                transition_contracts: (index < unit_count)
                    .then(|| ShootingTransitionContractV1 {
                        to_narrative_unit_index: index + 1,
                        transition_type: ShootingTransitionTypeV1::HardCut,
                        preserved_action: None,
                        preserved_sound: None,
                        preserved_knowledge: Vec::new(),
                        required_cut_point: None,
                    })
                    .into_iter()
                    .collect(),
                title: unit.title.clone(),
                slugline: format!("内景 场景{index} 夜"),
                location: None,
                time_of_day: None,
                participants: vec!["主角".into()],
                focal_character: None,
                source_span_ids: Vec::new(),
                visual_narrative_unit_id: Some(unit.id.clone()),
                user_addition_ids: unit
                    .shots
                    .iter()
                    .map(|shot| segment_addition_id(&shot.segment_id))
                    .collect(),
                dramatic_question: "能否通过".into(),
                scene_objective: "通过".into(),
                scene_obstacle: "黑暗".into(),
                relationship_before: "独自".into(),
                relationship_after: "独自".into(),
                audience_knowledge_before: Vec::new(),
                audience_knowledge_after: Vec::new(),
                withheld_information: Vec::new(),
                viewpoint: ShootingViewpointV1 {
                    mode: ShootingViewpointModeV1::LimitedThirdPerson,
                    anchor_character: None,
                    information_strategy: "跟随主角".into(),
                },
                spatial_continuity: ShootingSpatialContinuityV1 {
                    continuity_intent: ShootingContinuityIntentV1::Transparent,
                    primary_axis: None,
                    screen_direction: None,
                    reorientation_strategy: None,
                },
                sound_strategy: "安静".into(),
                entry_state: "进入".into(),
                exit_state: "离开".into(),
                shots: unit
                    .shots
                    .iter()
                    .enumerate()
                    .map(|(offset, planned)| shot(planned, offset as u32 + 1))
                    .collect(),
            }
        })
        .collect();
    let script = ChapterShootingScriptV1 {
        schema_version: ChapterShootingScriptSchemaVersion::V1,
        visual_plan_source: Some(source(1)),
        user_additions: projected.user_additions,
        directing_statement: Some(projected.directing_statement),
        scenes,
    };
    script.validate().expect("fixture script");
    script.validate_director_plan(plan, None).expect("fixture binds plan");
    script
}

fn rebase(
    script: &ChapterShootingScriptV1,
    plan: &ChapterVisualPlanV3,
) -> (ChapterShootingScriptV1, ShootingScriptPlanSyncV1) {
    let projected = project_visual_plan_for_shooting_script(plan, &[]).unwrap();
    let rebased = script.rebased_onto(&projected, source(2)).expect("rebase");
    rebased.validate().expect("rebased script keeps the stored contract");
    rebased
        .validate_director_plan(plan, Some(script))
        .expect("rebased script binds the new plan");
    let sync = rebased.plan_sync(&projected);
    (rebased, sync)
}

fn shot_of<'a>(script: &'a ChapterShootingScriptV1, id: &str) -> &'a ShootingShotV1 {
    script
        .scenes
        .iter()
        .flat_map(|scene| &scene.shots)
        .find(|shot| shot.shot_id == id)
        .unwrap()
}

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn unchanged_plan_is_current() {
    let plan = plan();
    let script = script(&plan);
    assert!(script.plan_sync_for(&plan).unwrap().is_current());
    let (rebased, sync) = rebase(&script, &plan);
    assert!(sync.is_current());
    assert_eq!(rebased.scenes, script.scenes);
}

#[test]
fn editing_one_shot_only_marks_that_shot_stale() {
    let plan = plan();
    let script = script(&plan);
    let mut edited = plan.clone();
    edited.segments[2].prompt = "主角翻窗而入".into();
    let (rebased, sync) = rebase(&script, &edited);

    assert_eq!(sync.stale_shot_ids, ids(&["shot-3"]));
    assert!(sync.missing_shot_ids.is_empty());
    assert_eq!(sync.stale_unit_ids, ids(&["unit:shot-3"]));
    // 过时镜头保留旧内容供参考；其他镜头与场景逐字节保留。
    assert_eq!(shot_of(&rebased, "shot-3"), shot_of(&script, "shot-3"));
    assert_eq!(rebased.scenes[0], script.scenes[0]);
    assert_eq!(shot_of(&rebased, "shot-4"), shot_of(&script, "shot-4"));
    assert_eq!(rebased.visual_plan_source, Some(source(2)));
    assert!(
        rebased
            .user_additions
            .iter()
            .any(|addition| addition.description == "主角翻窗而入")
    );
}

#[test]
fn retimed_shot_with_same_duration_stays_current_without_rewriting_constraints() {
    let plan = plan();
    let script = script(&plan);
    let mut edited = plan.clone();
    for (id, at_ms) in [("p3", 13_000), ("p2", 9_000)] {
        edited = edited
            .edited(&VisualPlanEditOperationV3::MovePoint {
                point_id: id.into(),
                at_ms,
            })
            .unwrap();
    }
    let (rebased, sync) = rebase(&script, &edited);

    assert_eq!(sync.stale_shot_ids, ids(&["shot-2", "shot-4"]));
    let moved = shot_of(&rebased, "shot-3");
    assert_eq!((moved.planned_start_ms, moved.planned_end_ms), (Some(9_000), Some(13_000)));
    assert_eq!(
        moved.visual_constraints,
        shot_of(&script, "shot-3").visual_constraints,
        "视频片段依赖的约束字节不能因平移而改变"
    );
}

#[test]
fn new_shots_are_missing_and_removed_shots_are_dropped() {
    let plan = plan();
    let script = script(&plan);
    let edited = plan
        .edited(&VisualPlanEditOperationV3::InsertPoint {
            point: point("p4", 14_000),
        })
        .unwrap()
        .edited(&VisualPlanEditOperationV3::SplitSegment {
            segment_id: "shot-4".into(),
            point_id: "p4".into(),
            right_segment_id: "shot-5".into(),
            left_prompt: "shot-4 的人物行动".into(),
            right_prompt: "新镜头".into(),
            left_scene_binding: Some(SceneBindingV3::Scene {
                scene_profile_id: 2,
            }),
            right_scene_binding: Some(SceneBindingV3::Scene {
                scene_profile_id: 2,
            }),
        })
        .unwrap()
        .edited(&VisualPlanEditOperationV3::MergeSegments {
            left_segment_id: "shot-1".into(),
            right_segment_id: "shot-2".into(),
            prompt: "shot-1 的人物行动".into(),
            scene_binding: Some(SceneBindingV3::Scene {
                scene_profile_id: 1,
            }),
        })
        .unwrap();
    let (rebased, sync) = rebase(&script, &edited);

    assert_eq!(sync.stale_shot_ids, ids(&["shot-1", "shot-4"]));
    assert_eq!(sync.missing_shot_ids, ids(&["shot-5"]));
    assert_eq!(sync.stale_unit_ids, ids(&["unit:shot-1", "unit:shot-3"]));
    assert!(
        rebased
            .scenes
            .iter()
            .flat_map(|scene| &scene.shots)
            .all(|shot| shot.shot_id != "shot-2")
    );
    assert!(
        rebased
            .user_additions
            .iter()
            .all(|addition| addition.id != "director:shot-2")
    );
}

#[test]
fn restructured_unit_is_left_for_generation_and_transitions_follow_their_targets() {
    let plan = plan();
    let script = script(&plan);
    let mut edited = plan.clone();
    edited.segments[1].scene_binding = Some(SceneBindingV3::Scene {
        scene_profile_id: 3,
    });
    let (rebased, sync) = rebase(&script, &edited);

    // shot-2 独立成新单元，原场景 1 只沿用到 shot-1 所在单元。
    assert_eq!(
        rebased
            .scenes
            .iter()
            .map(|scene| scene.scene_identity.as_str())
            .collect::<Vec<_>>(),
        vec!["unit:shot-1", "unit:shot-3"]
    );
    assert_eq!(sync.missing_shot_ids, ids(&["shot-2"]));
    assert_eq!(sync.stale_unit_ids, ids(&["unit:shot-2"]));
    assert_eq!(
        rebased
            .scenes
            .iter()
            .map(|scene| scene.narrative_unit_index)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(
        rebased.scenes[0].transition_contracts[0].to_narrative_unit_index,
        3
    );
    assert_eq!(rebased.scenes[1].timeline_position, 2);
}

#[test]
fn manual_rebind_resolves_only_the_confirmed_shot() {
    let plan = plan();
    let script = script(&plan);
    let mut edited = plan.clone();
    edited.segments[2].prompt = "主角翻窗而入".into();
    let (mut rebased, _) = rebase(&script, &edited);
    let projected = project_visual_plan_for_shooting_script(&edited, &[]).unwrap();

    assert!(
        rebased
            .clone()
            .rebind_shots(&projected, &ids(&["shot-9"]))
            .is_err()
    );
    let before = rebased.clone();
    rebased
        .rebind_shots(&projected, &ids(&["shot-3", "shot-4"]))
        .unwrap();
    assert!(rebased.plan_sync(&projected).is_current());
    assert_eq!(shot_of(&rebased, "shot-3").narrative_purpose, "主角翻窗而入");
    assert_eq!(shot_of(&rebased, "shot-4"), shot_of(&before, "shot-4"));
    rebased.validate().unwrap();
    rebased
        .validate_director_plan(&edited, Some(&before))
        .unwrap();
}

#[test]
fn stale_constraints_cannot_be_fabricated() {
    let plan = plan();
    let script = script(&plan);
    let mut edited = plan.clone();
    edited.segments[2].prompt = "主角翻窗而入".into();
    let (rebased, _) = rebase(&script, &edited);

    assert!(rebased.validate_director_plan(&edited, None).is_err());
    let mut forged = rebased.clone();
    forged.scenes[1].shots[0]
        .visual_constraints
        .as_mut()
        .unwrap()
        .prompt = "伪造的约束".into();
    assert!(
        forged
            .validate_director_plan(&edited, Some(&rebased))
            .is_err()
    );
}
