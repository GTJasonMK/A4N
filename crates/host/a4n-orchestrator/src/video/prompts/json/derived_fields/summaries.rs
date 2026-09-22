use crate::video::prompts::json::contracts::VideoSegmentPlan;

pub(super) fn character_summary(segment: &VideoSegmentPlan) -> String {
    segment
        .emotion_action_goals
        .first()
        .map(|goal| goal.character.clone())
        .unwrap_or_else(|| "无人物（纯环境镜头）".to_string())
}

pub(super) fn performance_summary(segment: &VideoSegmentPlan) -> String {
    let plan = &segment.performance_plan;
    [
        plan.movement_path.as_str(),
        plan.body_performance.as_str(),
        plan.gaze_and_hands.as_str(),
        plan.pause_or_reaction.as_str(),
    ]
    .join("；")
}

pub(super) fn action_goal_summary(segment: &VideoSegmentPlan) -> String {
    let summary = segment
        .emotion_action_goals
        .iter()
        .map(|goal| {
            format!(
                "{}：{}：{}",
                goal.character, goal.emotion, goal.visible_action
            )
        })
        .collect::<Vec<_>>()
        .join("；");
    if summary.is_empty() {
        "纯环境镜头，以环境动态、光影变化和摄影机运动承载变化".to_string()
    } else {
        summary
    }
}

pub(super) fn first_action_goal(segment: &VideoSegmentPlan) -> String {
    segment
        .emotion_action_goals
        .first()
        .map(|goal| {
            format!(
                "{}：{}：{}",
                goal.character, goal.emotion, goal.visible_action
            )
        })
        .or_else(|| {
            segment
                .performance_plan
                .time_blocks
                .first()
                .map(|block| block.subject_action.clone())
        })
        .unwrap_or_else(|| "建立环境状态".to_string())
}

pub(super) fn segment_end_summary(segment: &VideoSegmentPlan) -> String {
    segment.emotional_arc.end.clone()
}

pub(super) fn emotional_end_summary(segment: &VideoSegmentPlan) -> String {
    segment.emotional_arc.end.clone()
}

pub(super) fn compact_reference_ids(ids: &[i64]) -> String {
    if ids.is_empty() {
        return "参考一致性由文字约束承担".to_string();
    }
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn structured_arc_summary(value: &a4n_video_contract::VideoEmotionalArcV1) -> String {
    format!(
        "{} -> {} -> {} -> {}",
        value.start, value.turn, value.end, value.function
    )
}

pub(super) fn video_intent_summary(value: &a4n_video_contract::VideoIntentV1) -> String {
    format!(
        "{}；{}；{}；{}",
        value.hook, value.audience_effect, value.rhythm, value.visual_priority
    )
}

pub(super) fn material_coverage_summary(
    value: &a4n_video_contract::VideoMaterialCoverageV1,
) -> String {
    let required = value.required_entities.join("、");
    let missing = value.missing_or_weak_references.join("、");
    format!(
        "需要稳定：{}；弱覆盖：{}；风险：{}",
        if required.is_empty() {
            "无"
        } else {
            &required
        },
        if missing.is_empty() { "无" } else { &missing },
        value.risk_note
    )
}
