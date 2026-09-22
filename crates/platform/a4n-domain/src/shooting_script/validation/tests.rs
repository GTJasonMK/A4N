use crate::shooting_script::*;

fn valid_script() -> ChapterShootingScriptV1 {
    ChapterShootingScriptV1 {
        schema_version: ChapterShootingScriptSchemaVersion::V1,
        visual_plan_source: None,
        user_additions: Vec::new(),
        directing_statement: ShootingDirectingStatementV1 {
            central_dramatic_question: "主角能否确认异常".to_string(),
            protagonist_throughline: "克制恐惧并继续调查".to_string(),
            audience_experience: "逐步感到紧张".to_string(),
            viewpoint_strategy: "限制在主角感知范围".to_string(),
            visual_strategy: "保持空间关系清晰".to_string(),
            spatial_strategy: "由门口移动到暗廊".to_string(),
            sound_strategy: "用脚步和停顿制造悬念".to_string(),
            rhythm_strategy: "进入、停顿、观察".to_string(),
        },
        scenes: vec![ShootingSceneV1 {
            scene_index: 1,
            scene_identity: "hall".to_string(),
            sequence_index: 1,
            narrative_unit_index: 1,
            timeline_position: 1,
            intercut_group_id: None,
            editing_mode: ShootingEditingModeV1::Continuity,
            transition_contracts: Vec::new(),
            title: "进入大厅".to_string(),
            slugline: "内景 大厅 夜".to_string(),
            location: Some("大厅".to_string()),
            time_of_day: Some("夜".to_string()),
            participants: vec!["主角".to_string()],
            focal_character: Some("主角".to_string()),
            source_span_ids: vec!["span-1".to_string()],
            visual_narrative_unit_id: None,
            user_addition_ids: Vec::new(),
            dramatic_question: "暗处是否存在威胁".to_string(),
            scene_objective: "确认大厅状况".to_string(),
            scene_obstacle: "昏暗遮蔽视线".to_string(),
            relationship_before: "主角独自行动".to_string(),
            relationship_after: "主角意识到自己可能被观察".to_string(),
            audience_knowledge_before: vec!["大厅昏暗".to_string()],
            audience_knowledge_after: vec!["暗廊可能存在异常".to_string()],
            withheld_information: vec!["暗处人物身份".to_string()],
            viewpoint: ShootingViewpointV1 {
                mode: ShootingViewpointModeV1::LimitedThirdPerson,
                anchor_character: Some("主角".to_string()),
                information_strategy: "只展示主角可感知的信息".to_string(),
            },
            spatial_continuity: ShootingSpatialContinuityV1 {
                continuity_intent: ShootingContinuityIntentV1::Transparent,
                primary_axis: Some("门口到暗廊".to_string()),
                screen_direction: Some("由左向右".to_string()),
                reorientation_strategy: None,
            },
            sound_strategy: "突出脚步和停顿".to_string(),
            entry_state: "主角站在门口".to_string(),
            exit_state: "主角停在暗廊前".to_string(),
            shots: vec![ShootingShotV1 {
                shot_index: 1,
                shot_id: "shot-1".to_string(),
                title: "进入".to_string(),
                source_span_ids: vec!["span-1".to_string()],
                visual_shot_id: None,
                planned_start_ms: None,
                planned_end_ms: None,
                user_addition_ids: Vec::new(),
                visual_constraints: None,
                coverage: ShootingCoverageV1::MustShow,
                narrative_purpose: "建立空间并发现异常".to_string(),
                subject: "主角".to_string(),
                shot_size: ShootingShotSizeV1::Medium,
                camera_angle: ShootingCameraAngleV1::EyeLevel,
                camera_height: ShootingCameraHeightV1::Eye,
                lens_intent: "保持空间关系清晰".to_string(),
                camera_movement: ShootingCameraMovementV1::PushIn,
                composition: "主角位于画面左侧".to_string(),
                spatial_anchors: vec!["门口".to_string(), "暗廊".to_string()],
                blocking: ShootingBlockingV1 {
                    start: "门口".to_string(),
                    movement: "走向暗廊".to_string(),
                    end: "暗廊前".to_string(),
                },
                performance_beats: vec![ShootingPerformanceBeatV1 {
                    beat_index: 1,
                    beat_id: "beat-1".to_string(),
                    source_span_ids: vec!["span-1".to_string()],
                    user_addition_ids: Vec::new(),
                    actor: Some("主角".to_string()),
                    objective: "确认异常".to_string(),
                    obstacle: "视线受阻".to_string(),
                    visible_action: "放慢脚步观察暗廊".to_string(),
                    reaction: None,
                    expression: Some("紧张".to_string()),
                    gaze: Some("看向暗廊".to_string()),
                    pose_or_movement: Some("收紧肩膀".to_string()),
                    dialogue_intent: None,
                    audience_question: "暗处是否有人".to_string(),
                    answer_or_delay: "暂不揭示".to_string(),
                    knowledge_delta: vec!["暗廊存在异常".to_string()],
                    relationship_delta: None,
                    spatial_change: Some("从门口移动到暗廊".to_string()),
                    entry_state: "谨慎".to_string(),
                    exit_state: "警惕".to_string(),
                    required: true,
                    estimated_seconds: 2.0,
                }],
                dialogue_cues: Vec::new(),
                sound: ShootingSoundDesignV1 {
                    dialogue: Vec::new(),
                    environment: vec!["大厅回声".to_string()],
                    effects: vec!["脚步声".to_string()],
                    music: Vec::new(),
                    silence: vec!["观察前停顿".to_string()],
                },
                camera_start_state: "平视中景".to_string(),
                camera_end_state: "平视中景".to_string(),
                start_state: "主角在门口".to_string(),
                end_state: "主角在暗廊前".to_string(),
                transition_in: "从开门动作开始".to_string(),
                transition_out: "停在暗廊前".to_string(),
                continuity_requirements: vec!["暗廊保持在画面右侧".to_string()],
                estimated_natural_seconds: 2.0,
            }],
        }],
    }
}

#[test]
fn valid_shooting_script_satisfies_full_contract() {
    valid_script().validate().expect("valid shooting script");
}

#[test]
fn director_constraints_reject_changed_identity_timing_and_images() {
    use crate::chapter_visual_plan::{ChapterVisualPlanV3, SceneBindingV3};
    let mut plan = ChapterVisualPlanV3::starter();
    plan.segments[0].prompt = "推近门口".into();
    plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
        scene_profile_id: 1,
    });
    plan.points[0].image_id = Some(7);
    let director = plan.project().unwrap().remove(0);
    let mut script = valid_script();
    let shot = &mut script.scenes[0].shots[0];
    shot.shot_id = director.segment_id.clone();
    shot.visual_shot_id = Some(director.segment_id.clone());
    shot.planned_start_ms = Some(director.start_ms);
    shot.planned_end_ms = Some(director.end_ms);
    shot.estimated_natural_seconds = 10.0;
    shot.visual_constraints = Some(director);
    assert!(script.validate_director_plan(&plan).is_ok());
    let original = script.clone();
    script.scenes[0].shots[0].shot_id = "other-shot".into();
    assert!(script.validate_director_plan(&plan).is_err());
    script = original.clone();
    script.scenes[0].shots[0].estimated_natural_seconds = 9.0;
    assert!(script.validate_director_plan(&plan).is_err());
    script = original;
    script.scenes[0].shots[0]
        .visual_constraints
        .as_mut()
        .unwrap()
        .keyframes[0]
        .image_id = Some(8);
    assert!(script.validate_director_plan(&plan).is_err());
}

#[test]
fn scene_call_uses_the_publication_content_rules() {
    let script = valid_script();
    let scene = &script.scenes[0];
    assert!(
        scene
            .validate_content(&script.user_additions, false)
            .is_ok()
    );
    for field in ["slugline", "lens_intent", "beat_objective", "dialogue_beat"] {
        let mut scene = scene.clone();
        match field {
            "slugline" => scene.slugline.clear(),
            "lens_intent" => scene.shots[0].lens_intent.clear(),
            "beat_objective" => scene.shots[0].performance_beats[0].objective.clear(),
            "dialogue_beat" => scene.shots[0].dialogue_cues.push(ShootingDialogueCueV1 {
                beat_id: "missing".into(),
                speaker: "主角".into(),
                line_or_intent: "谁在那里".into(),
                listener_reaction: None,
            }),
            _ => unreachable!(),
        }
        assert!(
            scene
                .validate_content(&script.user_additions, false)
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn schema_version_rejects_noncanonical_values_during_deserialization() {
    let mut value = serde_json::to_value(valid_script()).unwrap();
    value["schema_version"] = serde_json::json!("legacy_shooting_script");

    assert!(serde_json::from_value::<ChapterShootingScriptV1>(value).is_err());
}

#[test]
fn transition_type_rejects_free_text_during_deserialization() {
    assert_eq!(
        serde_json::from_str::<ShootingTransitionTypeV1>("\"match_cut\"").unwrap(),
        ShootingTransitionTypeV1::MatchCut
    );
    assert!(serde_json::from_str::<ShootingTransitionTypeV1>("\"cut\"").is_err());
}

#[test]
fn shot_source_spans_must_belong_to_scene() {
    let mut script = valid_script();
    script.scenes[0].shots[0].source_span_ids = vec!["outside-scene".to_string()];

    let error = script.validate().expect_err("foreign shot span must fail");

    assert_eq!(error.path, "scenes[0].shots[0].source_span_ids");
    assert_eq!(error.message, "必须属于所在场景的来源范围");
}

#[test]
fn shot_duration_must_equal_performance_beats() {
    let mut script = valid_script();
    script.scenes[0].shots[0].estimated_natural_seconds = 3.0;

    let error = script.validate().expect_err("duration mismatch must fail");

    assert_eq!(error.path, "scenes[0].shots[0].estimated_natural_seconds");
    assert_eq!(error.message, "必须等于表演节拍时长之和");
}

#[test]
fn supporting_shot_may_contain_only_optional_beats() {
    let mut script = valid_script();
    script.scenes[0].shots[0].coverage = ShootingCoverageV1::ShouldShow;
    script.scenes[0].shots[0].performance_beats[0].required = false;

    script
        .validate()
        .expect("supporting shots do not have to cover a required performance fact");
}

#[test]
fn establishing_scene_may_have_no_participants() {
    let mut script = valid_script();
    script.scenes[0].participants.clear();
    script.scenes[0].focal_character = None;
    script.scenes[0].viewpoint.anchor_character = None;
    script.scenes[0].shots[0].performance_beats.clear();

    script
        .validate()
        .expect("environment-only scenes may omit participants and performance beats");
}
