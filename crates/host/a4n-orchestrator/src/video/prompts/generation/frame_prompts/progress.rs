use super::{FieldMilestone, StructuredJsonProgressSpec};

pub(super) const SEGMENT_PLAN_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("blocking_summary", "调度摘要", 1),
    FieldMilestone::new("emotion_action_goals", "情绪动作目标", 1),
    FieldMilestone::new("video_intent", "视频意图", 1),
    FieldMilestone::new("emotional_arc", "情绪弧线", 1),
    FieldMilestone::new("material_coverage", "素材覆盖", 1),
];

pub(super) const FRAME_PROMPT_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("frame_prompt", "画面提示词", 2),
    FieldMilestone::new("blocking_state", "调度状态", 1),
    FieldMilestone::new("lighting_state", "光影状态", 1),
    FieldMilestone::new("negative_prompt", "负向提示词", 1),
];

pub(super) const VIDEO_PROMPT_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("visual_prompt", "画面提示词", 2),
    FieldMilestone::new("motion_prompt", "动作运镜提示词", 2),
    FieldMilestone::new("negative_prompt", "负向提示词", 1),
    FieldMilestone::new("dialogue_cues", "对白提示", 1),
];

pub(super) const CHAT_VIDEO_PROMPT_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("chat_prompt", "Chat 视频提示词", 3),
    FieldMilestone::new("negative_prompt", "负向提示词", 1),
];

pub(super) fn segment_plan_progress_spec() -> StructuredJsonProgressSpec {
    StructuredJsonProgressSpec {
        root_array_key: "segments",
        item_label: "片段规划",
        expected_items: Some(1),
        milestones: SEGMENT_PLAN_PROGRESS_FIELDS,
    }
}

pub(super) fn frame_prompt_progress_spec(
    item_label: &'static str,
    expected_items: Option<usize>,
) -> StructuredJsonProgressSpec {
    StructuredJsonProgressSpec {
        root_array_key: "frames",
        item_label,
        expected_items,
        milestones: FRAME_PROMPT_PROGRESS_FIELDS,
    }
}

pub(super) fn video_prompt_progress_spec(is_chat_video: bool) -> StructuredJsonProgressSpec {
    if is_chat_video {
        StructuredJsonProgressSpec {
            root_array_key: "shots",
            item_label: "Chat 视频提示词",
            expected_items: Some(1),
            milestones: CHAT_VIDEO_PROMPT_PROGRESS_FIELDS,
        }
    } else {
        StructuredJsonProgressSpec {
            root_array_key: "shots",
            item_label: "视频提示词",
            expected_items: Some(1),
            milestones: VIDEO_PROMPT_PROGRESS_FIELDS,
        }
    }
}
