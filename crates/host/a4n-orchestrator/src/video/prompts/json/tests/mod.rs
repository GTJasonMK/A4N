use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;

include!("fixtures.rs");
include!("prompt_contracts.rs");
include!("dialogue.rs");

#[test]
fn derives_environment_shot_source_spans_without_beats() {
    let mut script = crate::video::tests::test_director_bound_shooting_script("ch1:s1:f1", 2);
    script.scenes[0].shots[0].performance_beats.clear();
    let projection = crate::video::prompts::generation::video_projection_from_shooting_script(
        &script,
        a4n_video_contract::VideoShootingScriptIdentityV1 {
            record_id: 1,
            revision: 1,
            source_revision_key: "revision-1".to_string(),
            source_content_sha256: "a".repeat(64),
            script_sha256: "b".repeat(64),
            shot_fingerprints: Default::default(),
        },
    )
    .expect("environment shot should project");

    let ids = derive_segment_source_span_ids(&projection, "shot-1", &[], 1)
        .expect("environment shot should use shot source spans");

    assert_eq!(ids, vec!["ch1:s1:f1"]);
}

#[test]
fn validates_environment_segment_without_fake_character() {
    let mut script = crate::video::tests::test_director_bound_shooting_script("ch1:s1:f1", 6);
    script.scenes[0].shots.truncate(1);
    let mut projection = crate::video::prompts::generation::video_projection_from_shooting_script(
        &script,
        a4n_video_contract::VideoShootingScriptIdentityV1 {
            record_id: 1,
            revision: 1,
            source_revision_key: "revision-1".to_string(),
            source_content_sha256: "a".repeat(64),
            script_sha256: "b".repeat(64),
            shot_fingerprints: Default::default(),
        },
    )
    .unwrap();
    projection.scenes[0].participants.clear();
    projection.scenes[0].focal_character = None;
    projection.scenes[0].shots[0].participants.clear();
    projection.scenes[0].shots[0].focal_character = None;
    projection.scenes[0].shots[0].action_beats.clear();

    let mut segment = valid_segments().remove(0);
    segment.source_shot_id = "shot-1".to_string();
    segment.covered_beat_ids.clear();
    segment.emotion_action_goals.clear();
    segment.performance_plan.action_script.clear();
    segment.director_constraints = Some(
        projection
            .shot("shot-1")
            .unwrap()
            .director_constraints
            .clone(),
    );
    let segment = validate_segment_plan(segment).unwrap();
    validate_segment_plan_against_shooting_script_projection(
        std::slice::from_ref(&segment),
        &projection,
    )
    .unwrap();

    let input = serde_json::from_value(valid_shot_value(1)).unwrap();
    let derived = super::derived_fields::video_prompt_shot_from_input(input, 1, &segment);
    assert_eq!(derived.start_state["character"], "无人物（纯环境镜头）");
    assert_eq!(
        derived.transition_in["first_beat_action"],
        segment.performance_plan.time_blocks[0].subject_action
    );
    assert_eq!(
        derived.emotion_action_beats,
        json!(segment.performance_plan.time_blocks)
    );

    let mut invalid = segment;
    invalid
        .emotion_action_goals
        .push(a4n_video_contract::VideoEmotionActionGoalV1 {
            character: "青城夜景".to_string(),
            emotion: "安静".to_string(),
            visible_action: "灯光闪烁".to_string(),
        });
    let error = validate_segment_plan_against_shooting_script_projection(&[invalid], &projection)
        .unwrap_err();
    assert!(error.to_string().contains("纯环境镜头"));
}

#[test]
fn director_constrained_followup_shot_keeps_repeat_context_non_empty() {
    let mut segment = valid_segments().remove(0);
    let mut visual_plan = a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter();
    visual_plan.segments[0].prompt = "导演约束起点画面".to_string();
    visual_plan.segments[0].scene_binding =
        Some(a4n_domain::chapter_visual_plan::SceneBindingV3::Scene {
            scene_profile_id: 1,
        });
    segment.segment_number = 2;
    segment.director_constraints = Some(visual_plan.project().unwrap().remove(0));
    let segments = vec![segment];
    let raw = response_from_shots(vec![valid_shot_value(2)]);

    let shots = parse_video_prompt_shots(&raw, &segment_map(&segments), PromptOutputLanguage::ZhCn)
        .expect("director-constrained follow-up shot should validate");

    assert_eq!(
        shots[0].self_contained_context["must_repeat_from_previous"],
        json!(["不自动复述上一片段尾态；以导演编排提供的当前起点画面和约束为准"])
    );
}

#[test]
fn participant_segment_still_requires_character_actions() {
    let mut script = crate::video::tests::test_director_bound_shooting_script("ch1:s1:f1", 6);
    script.scenes[0].shots.truncate(1);
    let projection = crate::video::prompts::generation::video_projection_from_shooting_script(
        &script,
        a4n_video_contract::VideoShootingScriptIdentityV1 {
            record_id: 1,
            revision: 1,
            source_revision_key: "revision-1".to_string(),
            source_content_sha256: "a".repeat(64),
            script_sha256: "b".repeat(64),
            shot_fingerprints: Default::default(),
        },
    )
    .unwrap();
    let mut segment = valid_segments().remove(0);
    segment.source_shot_id = "shot-1".to_string();
    segment.covered_beat_ids = vec!["beat-1".to_string(), "beat-2".to_string()];
    segment.emotion_action_goals.clear();
    segment.performance_plan.action_script.clear();
    segment.director_constraints = Some(
        projection
            .shot("shot-1")
            .unwrap()
            .director_constraints
            .clone(),
    );

    let segment = validate_segment_plan(segment).unwrap();

    let error = validate_segment_plan_against_shooting_script_projection(&[segment], &projection)
        .unwrap_err();
    assert!(error.to_string().contains("emotion_action_goals 不能为空"));
}

#[test]
fn stored_director_segment_uses_director_frame_instead_of_request_default() {
    let mut segment = valid_segments().remove(0);
    let mut plan = a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter();
    plan.frame.width = 1280;
    plan.frame.height = 960;
    plan.points.last_mut().unwrap().at_ms = 6000;
    plan.segments[0].id = segment.source_shot_id.clone();
    plan.segments[0].prompt = "保留导演画幅".into();
    plan.segments[0].scene_binding = Some(a4n_domain::chapter_visual_plan::SceneBindingV3::Scene {
        scene_profile_id: 1,
    });
    segment.director_constraints = Some(plan.project().unwrap().remove(0));
    segment.source_span_ids.clear();
    segment.reference_image_ids.clear();
    segment.aspect_ratio = "4:3".into();
    let stored = validate_stored_video_segment_plan(vec![segment.clone()], &[], "16:9").unwrap();
    assert_eq!(stored[0], segment);
    segment.aspect_ratio = "16:9".into();
    assert!(validate_stored_video_segment_plan(vec![segment], &[], "16:9").is_err());
}
