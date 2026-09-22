use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;

include!("fixtures.rs");
include!("prompt_contracts.rs");
include!("dialogue.rs");

#[test]
fn derives_environment_shot_source_spans_without_beats() {
    let mut script = crate::video::tests::test_chapter_shooting_script("ch1:s1:f1");
    script.scenes[0].shots[0].performance_beats.clear();
    script.scenes[0].shots[0].estimated_natural_seconds = 2.0;
    let projection = crate::video::prompts::generation::video_projection_from_shooting_script(
        &script,
        a4n_video_contract::VideoShootingScriptIdentityV1 {
            record_id: 1,
            revision: 1,
            source_revision_key: "revision-1".to_string(),
            source_content_sha256: "a".repeat(64),
            script_sha256: "b".repeat(64),
        },
        5,
    )
    .expect("environment shot should project");

    let ids = derive_segment_source_span_ids(&projection, "shot-1", &[], 1)
        .expect("environment shot should use shot source spans");

    assert_eq!(ids, vec!["ch1:s1:f1"]);
}

#[test]
fn validates_environment_segment_without_fake_character() {
    let mut script = crate::video::tests::test_chapter_shooting_script("ch1:s1:f1");
    script.scenes[0].shots.truncate(1);
    let mut projection = crate::video::prompts::generation::video_projection_from_shooting_script(
        &script,
        a4n_video_contract::VideoShootingScriptIdentityV1 {
            record_id: 1,
            revision: 1,
            source_revision_key: "revision-1".to_string(),
            source_content_sha256: "a".repeat(64),
            script_sha256: "b".repeat(64),
        },
        6,
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
fn participant_segment_still_requires_character_actions() {
    let mut script = crate::video::tests::test_chapter_shooting_script("ch1:s1:f1");
    script.scenes[0].shots.truncate(1);
    let projection = crate::video::prompts::generation::video_projection_from_shooting_script(
        &script,
        a4n_video_contract::VideoShootingScriptIdentityV1 {
            record_id: 1,
            revision: 1,
            source_revision_key: "revision-1".to_string(),
            source_content_sha256: "a".repeat(64),
            script_sha256: "b".repeat(64),
        },
        6,
    )
    .unwrap();
    let mut segment = valid_segments().remove(0);
    segment.source_shot_id = "shot-1".to_string();
    segment.covered_beat_ids = vec!["beat-1".to_string(), "beat-2".to_string()];
    segment.emotion_action_goals.clear();
    segment.performance_plan.action_script.clear();
    let segment = validate_segment_plan(segment).unwrap();

    let error = validate_segment_plan_against_shooting_script_projection(&[segment], &projection)
        .unwrap_err();
    assert!(error.to_string().contains("emotion_action_goals 不能为空"));
}
