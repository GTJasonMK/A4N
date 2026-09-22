use a4n_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};

use crate::structured_llm::StructuredLlmProgress;

const LAYOUT_UNIT_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("transition_contracts", "转场合同", 2),
    FieldMilestone::new("title", "场景标题", 1),
    FieldMilestone::new("slugline", "场景标头", 1),
    FieldMilestone::new("participants", "参与角色", 1),
    FieldMilestone::new("dramatic_question", "戏剧问题", 1),
    FieldMilestone::new("scene_objective", "场景目标", 1),
    FieldMilestone::new("scene_obstacle", "场景阻碍", 1),
    FieldMilestone::new("exit_state", "结束状态", 1),
];

const UNIT_FACT_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("actor", "表演者", 1),
    FieldMilestone::new("objective", "行动目标", 1),
    FieldMilestone::new("obstacle", "行动阻力", 1),
    FieldMilestone::new("visible_action", "可见动作", 2),
    FieldMilestone::new("reaction", "可见反应", 1),
    FieldMilestone::new("dialogue_intent", "对白意图", 1),
    FieldMilestone::new("exit_state", "结束状态", 1),
];

const DESIGN_SHOT_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "镜头标题", 1),
    FieldMilestone::new("narrative_purpose", "叙事目的", 1),
    FieldMilestone::new("camera_movement", "摄影运动", 1),
    FieldMilestone::new("blocking", "演员调度", 2),
    FieldMilestone::new("performance_beats", "表演节拍", 4),
    FieldMilestone::new("dialogue_cues", "对白提示", 1),
    FieldMilestone::new("sound", "声音设计", 1),
    FieldMilestone::new("continuity_requirements", "连续性要求", 1),
];

pub(super) const SKELETON_STREAM_SPAN: ProgressSpan = ProgressSpan::new(2, 6);
pub(super) const LAYOUT_UNIT_STREAM_SPAN: ProgressSpan = ProgressSpan::new(6, 16);
pub(super) const UNIT_ADAPTATION_STREAM_SPAN: ProgressSpan = ProgressSpan::new(18, 46);
pub(super) const DESIGN_STREAM_SPAN: ProgressSpan = ProgressSpan::new(50, 90);

pub(super) fn layout_unit_stream_progress(
    unit_offset: usize,
    unit_count: usize,
) -> StructuredLlmProgress {
    StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "transition_contracts",
            item_label: "转场合同",
            expected_items: None,
            milestones: LAYOUT_UNIT_FIELDS,
        },
        span: LAYOUT_UNIT_STREAM_SPAN.split(unit_offset, unit_count),
    }
}

pub(super) fn unit_adaptation_stream_progress(
    unit_offset: usize,
    unit_count: usize,
) -> StructuredLlmProgress {
    StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "performance_facts",
            item_label: "表演事实",
            expected_items: None,
            milestones: UNIT_FACT_FIELDS,
        },
        span: UNIT_ADAPTATION_STREAM_SPAN.split(unit_offset, unit_count),
    }
}

pub(super) fn design_unit_stream_progress(
    unit_offset: usize,
    unit_count: usize,
) -> StructuredLlmProgress {
    StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "shots",
            item_label: "镜头",
            expected_items: None,
            milestones: DESIGN_SHOT_FIELDS,
        },
        span: DESIGN_STREAM_SPAN.split(unit_offset, unit_count),
    }
}

pub(super) fn design_shot_stream_progress(
    unit_offset: usize,
    unit_count: usize,
    shot_offset: usize,
    shot_count: usize,
) -> StructuredLlmProgress {
    let unit_span = DESIGN_STREAM_SPAN.split(unit_offset, unit_count);
    StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "shots",
            item_label: "镜头",
            expected_items: Some(1),
            milestones: DESIGN_SHOT_FIELDS,
        },
        span: unit_span.split(shot_offset, shot_count),
    }
}
