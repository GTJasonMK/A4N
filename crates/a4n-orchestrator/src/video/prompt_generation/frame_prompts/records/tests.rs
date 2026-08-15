use super::*;
use std::collections::{HashMap, HashSet};

use crate::video::PromptOutputLanguage;
use crate::video::prompt_generation::frame_prompts::VideoUnitPlan;
use crate::video::prompt_json::{
    StoryboardPanel, VideoFramePrompt, VideoPromptShot, VideoSegmentPlan,
};
use crate::video::prompt_store::NewChapterVideoPrompt;
use a4n_core::error::AppError;
use serde_json::{Value, json};

fn segment(segment_number: i32) -> VideoSegmentPlan {
    serde_json::from_value(json!({
        "schema_version": a4n_core::video_execution::VIDEO_SEGMENT_PLAN_V2,
        "segment_number": segment_number,
        "source_shot_id": format!("shot-{segment_number}"),
        "segment_index_in_shot": 1,
        "shot_segment_count": 1,
        "covered_beat_ids": [format!("shot-{segment_number}-beat-1")],
        "title": format!("片段 {segment_number}"),
        "scene_summary": "片段摘要",
        "blocking_summary": "站位摘要",
        "spatial_anchors": [{"anchor": "door", "usage": "空间参照"}],
        "emotion_action_goals": [{"character": "角色", "emotion": "克制", "visible_action": "停住"}],
        "video_intent": {
            "hook": "片段钩子",
            "audience_effect": "观众感受",
            "rhythm": "稳定",
            "visual_priority": "主体动作"
        },
        "emotional_arc": {
            "start": "开始",
            "turn": "转折",
            "end": "结束",
            "function": "测试"
        },
        "performance_plan": {
            "movement_path": "从起点移动到终点",
            "body_performance": "身体姿态随动作变化",
            "gaze_and_hands": "视线和手部跟随目标变化",
            "pause_or_reaction": "结尾停顿",
            "camera_relationship": "镜头保持主体动作清晰",
            "time_blocks": [{"start_sec": 0.0, "end_sec": 6.0, "camera": "固定中景", "subject_action": "角色完成动作", "continuity_target": "稳定尾态"}],
            "action_script": [{"character": "角色", "body": "重心稳定", "movement": "走到终点", "hands": "双手稳定", "gaze": "看向目标", "timing": "6 秒内"}]
        },
        "material_coverage": {
            "required_entities": ["角色"],
            "missing_or_weak_references": [],
            "risk_note": "无"
        },
        "transition_to_next": null,
        "source_span_ids": ["ch1:s1:f1"],
        "duration_secs": 6,
        "aspect_ratio": "16:9",
        "reference_image_ids": []
    }))
    .expect("typed segment fixture should deserialize")
}

fn frame(frame_number: i32) -> VideoFramePrompt {
    VideoFramePrompt {
        frame_number,
        title: format!("帧 {frame_number}"),
        scene_summary: "帧摘要".to_string(),
        storyboard_panels: Vec::new(),
        frame_prompt: "完整帧提示词".to_string(),
        blocking_state: json!({"blocking": "stable"}),
        lighting_state: json!({"lighting": "warm"}),
        negative_prompt: "不要变形".to_string(),
        duration_secs: 6,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
        segment_numbers: vec![frame_number.max(1)],
    }
}

fn storyboard_panel(panel_index: i32) -> StoryboardPanel {
    StoryboardPanel {
        panel_index,
        moment: format!("第 {panel_index} 个状态"),
        visual_goal: "表达可见动作变化".to_string(),
        composition: "保持主体和空间关系清晰".to_string(),
        continuity_note: "延续人物、场景和光源方向".to_string(),
    }
}

fn video_unit_for_segment(segment: &VideoSegmentPlan) -> VideoUnitPlan {
    super::super::video_units_for_segment_plan(std::slice::from_ref(segment), segment.duration_secs)
        .expect("single segment video unit should fit i32 sequence ranges")
        .remove(0)
}

fn shot(shot_number: i32) -> VideoPromptShot {
    VideoPromptShot {
        shot_number,
        title: format!("镜头 {shot_number}"),
        scene_summary: "镜头摘要".to_string(),
        self_contained_context: json!({
            "must_repeat_from_previous": ["人物身份", "空间锚点"],
            "must_establish_in_segment": ["本段动作目标"],
            "must_preserve_throughout": ["光影方向"],
            "handoff_to_next": "结尾状态",
            "omission_risks": ["上下段断裂"]
        }),
        start_state: json!({"state": "start"}),
        end_state: json!({"state": "end"}),
        transition_in: json!({
            "from_previous_end": "承接上一状态",
            "to_current_start": "进入本段起点",
            "cut_type": "连续承接",
            "camera_transition": "稳定衔接",
            "first_beat_action": "人物开始动作"
        }),
        visual_prompt: "完整 visual prompt".to_string(),
        motion_prompt: "完整 motion prompt".to_string(),
        negative_prompt: "不要变形".to_string(),
        blocking_plan: json!({"blocking": "plan"}),
        lighting_plan: json!({"lighting": "plan"}),
        emotion_action_beats: json!(["beat"]),
        dialogue_beats: json!([]),
        cinematography_plan: json!({"camera": "push"}),
        temporal_beats: json!(["start", "end"]),
        reference_binding: json!({"bindings": [], "coverage_check": "参考一致性由文字约束承担"}),
        continuity_check: json!({"revision": "无需修订"}),
        duration_secs: 6,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
    }
}

fn test_metadata() -> Value {
    json!({"source": {"content_sha256": "current"}})
}

fn test_common_input<'a>(
    source_metadata: &'a Value,
    available_assets: &'a Value,
    generation_asset_fingerprint: &'a Value,
) -> VideoPromptDraftCommonInput<'a> {
    VideoPromptDraftCommonInput {
        project_id: 1,
        chapter_id: 2,
        style: "电影感写实",
        prompt_output_language: PromptOutputLanguage::ZhCn,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
        execution_identity: source_metadata,
    }
}

fn expect_invalid_argument(result: Result<Vec<NewChapterVideoPrompt>, AppError>) -> String {
    match result {
        Ok(_) => panic!("draft construction should fail"),
        Err(AppError::InvalidArgument(message)) => message,
        Err(other) => panic!("unexpected error: {other}"),
    }
}

#[test]
fn boundary_frame_prompts_reject_storyboard_panels() {
    let segment = segment(1);
    let mut start_frame = frame(1);
    start_frame.segment_numbers = vec![1];
    start_frame.storyboard_panels = vec![storyboard_panel(1), storyboard_panel(2)];
    let mut end_frame = frame(2);
    end_frame.segment_numbers = vec![1];

    let err = validate_frame_prompts(&[start_frame, end_frame], &[segment], &HashSet::new())
        .expect_err("boundary frames must reject storyboard panels");

    assert!(
        matches!(&err, AppError::InvalidArgument(message) if message.contains("不能输出 storyboard_panels")),
        "unexpected error: {err}"
    );
}

#[test]
fn storyboard_prompts_accept_dynamic_panel_count() {
    let segment = segment(1);
    let mut frame = frame(1);
    frame.segment_numbers = vec![1];
    frame.storyboard_panels = vec![
        storyboard_panel(1),
        storyboard_panel(2),
        storyboard_panel(3),
    ];
    frame.frame_prompt =
        "第 1 格：建立起始状态。\n第 2 格：表现核心动作。\n第 3 格：稳定到结尾状态。".to_string();

    validate_storyboard_prompts(&[frame], &[segment], &HashSet::new())
        .expect("valid storyboard panels should pass");
}

#[test]
fn storyboard_prompts_require_frame_prompt_panel_markers() {
    let segment = segment(1);
    let mut frame = frame(1);
    frame.segment_numbers = vec![1];
    frame.storyboard_panels = vec![storyboard_panel(1), storyboard_panel(2)];
    frame.frame_prompt = "第 1 格：建立起始状态。第二格表现核心动作。".to_string();

    let err = validate_storyboard_prompt(&frame, &[segment], &HashSet::new())
        .expect_err("missing panel marker should fail");

    assert!(
        matches!(&err, AppError::InvalidArgument(message) if message.contains("缺少第 2 格标记")),
        "unexpected error: {err}"
    );
}

#[test]
fn targeted_storyboard_prompt_uses_target_content_segment_after_action_bridge() {
    let mut first = segment(1);
    first.transition_to_next = Some(a4n_core::video_execution::VideoSegmentTransitionV1 {
        transition_type: a4n_domain::shooting_script::ShootingTransitionTypeV1::MatchCut,
        editing_mode: a4n_domain::shooting_script::ShootingEditingModeV1::Continuity,
        preserved_action: Some("角色推门动作连续完成".to_string()),
        preserved_sound: None,
        required_cut_point: None,
    });
    let plan = vec![first, segment(2)];
    let unit =
        super::super::video_unit_for_target_segment(&plan, 6, 2).expect("target storyboard unit");
    let mut frame = frame(2);
    frame.segment_numbers = vec![2];
    frame.storyboard_panels = vec![storyboard_panel(1), storyboard_panel(2)];
    frame.frame_prompt = "第 1 格：建立起始状态。\n第 2 格：表现核心动作。".to_string();

    validate_storyboard_prompt_for_unit(&frame, &plan, &unit, &HashSet::new())
        .expect("target segment 2 must not be mistaken for the preceding action bridge");
}

#[test]
fn video_prompt_drafts_reject_missing_boundary_frame_metadata() {
    let segment = segment(1);
    let mut segments = HashMap::new();
    segments.insert(1, &segment);
    let mut video_units = HashMap::new();
    video_units.insert(1, video_unit_for_segment(&segment));
    let mut frames = HashMap::new();
    frames.insert(1, frame(1));

    let source_metadata = test_metadata();
    let available_assets = json!({});
    let generation_asset_fingerprint = json!({"schema_version": 1});
    let message = expect_invalid_argument(video_prompt_drafts(VideoPromptDraftsInput {
        common: test_common_input(
            &source_metadata,
            &available_assets,
            &generation_asset_fingerprint,
        ),
        segment_by_number: &segments,
        video_unit_by_number: &video_units,
        frame_by_number: &frames,
        video_prompts: &[shot(1)],
    }));

    assert!(message.contains("缺少结束边界帧 #2"));
}

#[test]
fn first_frame_video_prompt_drafts_reject_missing_initial_first_frame() {
    let segment = segment(1);
    let mut segments = HashMap::new();
    segments.insert(1, &segment);
    let mut video_units = HashMap::new();
    video_units.insert(1, video_unit_for_segment(&segment));
    let frames = HashMap::new();

    let source_metadata = test_metadata();
    let available_assets = json!({});
    let generation_asset_fingerprint = json!({"schema_version": 1});
    let message = expect_invalid_argument(first_frame_video_prompt_drafts(
        FirstFrameVideoPromptDraftsInput {
            common: test_common_input(
                &source_metadata,
                &available_assets,
                &generation_asset_fingerprint,
            ),
            source_prompt_type: "frame_segment_plan",
            segment_by_number: &segments,
            video_unit_by_number: &video_units,
            first_frame_by_number: &frames,
            video_prompts: &[shot(1)],
        },
    ));

    assert!(message.contains("缺少初始首帧 #1"));
}

#[test]
fn first_frame_video_prompt_drafts_allow_later_target_without_initial_first_frame() {
    let segment = segment(2);
    let mut segments = HashMap::new();
    segments.insert(2, &segment);
    let mut video_units = HashMap::new();
    video_units.insert(
        2,
        super::super::video_unit_for_target_segment(
            std::slice::from_ref(&segment),
            segment.duration_secs,
            2,
        )
        .expect("target unit"),
    );

    let source_metadata = test_metadata();
    let available_assets = json!({});
    let generation_asset_fingerprint = json!({"schema_version": 1});
    let drafts = first_frame_video_prompt_drafts(FirstFrameVideoPromptDraftsInput {
        common: test_common_input(
            &source_metadata,
            &available_assets,
            &generation_asset_fingerprint,
        ),
        source_prompt_type: "frame_segment_plan",
        segment_by_number: &segments,
        video_unit_by_number: &video_units,
        first_frame_by_number: &HashMap::new(),
        video_prompts: &[shot(2)],
    })
    .expect("later continuity target must not require the chapter's initial system frame");

    assert_eq!(drafts.len(), 1);
    assert!(drafts[0].matches_identity(
        crate::video::prompt_store::PromptWorkflowMode::FirstFrameChain,
        "video_prompt",
        2,
    ));
}
