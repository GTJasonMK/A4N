use serde_json::{Value, json};

use crate::video::prompts::json::contracts::{
    VideoPromptShot, VideoPromptShotInput, VideoSegmentPlan,
};

use super::summaries::{
    action_goal_summary, character_summary, compact_reference_ids, emotional_end_summary,
    first_action_goal, material_coverage_summary, performance_summary, segment_end_summary,
    structured_arc_summary, video_intent_summary,
};

pub(in crate::video::prompts::json) fn video_prompt_shot_from_input(
    input: VideoPromptShotInput,
    shot_number: i32,
    segment: &VideoSegmentPlan,
) -> VideoPromptShot {
    VideoPromptShot {
        shot_number,
        title: input.title,
        scene_summary: input.scene_summary,
        self_contained_context: derive_self_contained_context(segment),
        start_state: derive_start_state(segment),
        end_state: derive_end_state(segment),
        transition_in: derive_transition_in(segment),
        visual_prompt: input.visual_prompt,
        motion_prompt: input.motion_prompt,
        negative_prompt: input.negative_prompt,
        blocking_plan: derive_blocking_plan(segment),
        lighting_plan: derive_lighting_plan(segment),
        emotion_action_beats: derive_emotion_action_beats(segment),
        dialogue_beats: input.dialogue_cues,
        cinematography_plan: derive_cinematography_plan(segment),
        temporal_beats: derive_temporal_beats(segment),
        reference_binding: derive_reference_binding(segment),
        continuity_check: derive_continuity_check(segment),
        duration_secs: segment.duration_secs,
        aspect_ratio: segment.aspect_ratio.clone(),
        reference_image_ids: segment.reference_image_ids.clone(),
    }
}

fn derive_self_contained_context(segment: &VideoSegmentPlan) -> Value {
    let repeat = if segment.segment_number == 1 || segment.director_constraints.is_some() {
        json!([])
    } else {
        json!(["承接上一片段尾态的人物身份、服装、空间锚点、道具状态、光源方向和镜头轴线"])
    };
    let performance_omission_risk = if segment.emotion_action_goals.is_empty() {
        "省略环境动态、光影变化或摄影机运动会削弱片段结尾承接"
    } else {
        "省略情绪动作目标会削弱片段结尾承接"
    };
    json!({
        "must_repeat_from_previous": repeat,
        "must_establish_in_segment": [
            segment.scene_summary,
            segment.blocking_summary,
            format!("片段目标：{}", video_intent_summary(&segment.video_intent)),
            format!(
                "情绪弧线：{}",
                structured_arc_summary(&segment.emotional_arc)
            )
        ],
        "must_preserve_throughout": [
            "人物身份、服装、固定参照物、左右/前后关系、主要道具和光源方向必须连续",
            format!("参考图约束：{}", compact_reference_ids(&segment.reference_image_ids))
        ],
        "handoff_to_next": segment_end_summary(segment),
        "omission_risks": [
            "省略分段规划中的空间锚点会造成段间站位跳变",
            performance_omission_risk
        ]
    })
}

fn derive_start_state(segment: &VideoSegmentPlan) -> Value {
    json!({
        "character": character_summary(segment),
        "position": segment.blocking_summary,
        "pose": action_goal_summary(segment),
        "camera": "按当前片段 visual_prompt 的起始构图和镜头说明执行",
        "lighting": "继承 visual_prompt 中描述的光源方向和阴影状态"
    })
}

fn derive_end_state(segment: &VideoSegmentPlan) -> Value {
    json!({
        "character": character_summary(segment),
        "position": segment_end_summary(segment),
        "pose": emotional_end_summary(segment),
        "camera": "最后一个镜头稳定到 visual_prompt 的结尾构图",
        "lighting": "结尾保持与片段起点连续的光源方向和阴影状态"
    })
}

fn derive_transition_in(segment: &VideoSegmentPlan) -> Value {
    json!({
        "from_previous_end": if segment.director_constraints.is_some() {
            "由本段起点图片建立画面；没有显式关联时，不从上一镜头的尾帧推导当前画面".to_string()
        } else if segment.segment_number == 1 {
            "第一片段从章节当前起点建立画面，不编造上一段尾态".to_string()
        } else {
            "承接上一片段尾态或已存在输入首帧的站位、道具和光影".to_string()
        },
        "to_current_start": segment.blocking_summary,
        "cut_type": "连续承接",
        "camera_transition": "按 visual_prompt 与 motion_prompt 的起始镜头说明进入当前片段",
        "first_beat_action": first_action_goal(segment)
    })
}

fn derive_blocking_plan(segment: &VideoSegmentPlan) -> Value {
    json!({
        "start": segment.blocking_summary,
        "movement": performance_summary(segment),
        "end": segment_end_summary(segment),
        "spatial_anchors": segment.spatial_anchors,
        "do_not_change": [
            "固定参考物的相对位置",
            "人物与主要道具、前后左右关系和距离逻辑"
        ]
    })
}

fn derive_lighting_plan(_segment: &VideoSegmentPlan) -> Value {
    json!({
        "key_light": "以 visual_prompt 中描述的主光为准",
        "direction": "全片段保持 visual_prompt 中声明的光源方向",
        "shadow": "阴影状态随人物动作自然变化但不跳变",
        "continuity": "起点到结尾保持同一光源逻辑，除非 visual_prompt 明确说明原文中的换场或时间变化"
    })
}

fn derive_emotion_action_beats(segment: &VideoSegmentPlan) -> Value {
    if segment.performance_plan.action_script.is_empty() {
        json!(segment.performance_plan.time_blocks)
    } else {
        json!(segment.performance_plan.action_script)
    }
}

fn derive_cinematography_plan(segment: &VideoSegmentPlan) -> Value {
    json!({
        "shot_size": "按 visual_prompt 的构图景别执行",
        "camera_angle": "按 visual_prompt 的机位视角执行",
        "camera_movement": "按 motion_prompt 的运镜说明执行",
        "axis_rule": "保持片段规划中的左右/前后关系和空间锚点连续",
        "lens_hint": format!("画幅 {}，片段时长 {} 秒", segment.aspect_ratio, segment.duration_secs)
    })
}

fn derive_temporal_beats(segment: &VideoSegmentPlan) -> Value {
    json!(segment.performance_plan.time_blocks)
}

fn derive_reference_binding(segment: &VideoSegmentPlan) -> Value {
    let bindings = segment
        .reference_image_ids
        .iter()
        .enumerate()
        .map(|(index, image_id)| {
            json!({
                "image_id": image_id,
                "entity": format!("系统绑定参考图 {}", index + 1),
                "role": "reference",
                "usage": "约束当前片段中的人物、场景、道具或风格一致性",
                "must_keep": "按 available_assets 中对应参考图保持外观、空间、道具或风格一致"
            })
        })
        .collect::<Vec<_>>();
    let coverage_check = if segment.reference_image_ids.is_empty() {
        "参考一致性由 visual_prompt 的文字约束承担".to_string()
    } else {
        format!(
            "已由系统为当前视频提示词绑定 {} 张参考图；素材覆盖信息：{}",
            segment.reference_image_ids.len(),
            material_coverage_summary(&segment.material_coverage)
        )
    };
    json!({
        "bindings": bindings,
        "coverage_check": coverage_check
    })
}

fn derive_continuity_check(segment: &VideoSegmentPlan) -> Value {
    json!({
        "blocking": "由 current_segment.blocking_summary、spatial_anchors 和 visual_prompt 共同约束站位连续",
        "lighting": "由 visual_prompt 和 lighting_plan 共同约束光影连续",
        "emotion": format!(
            "情绪弧线已由分段规划提供：{}",
            structured_arc_summary(&segment.emotional_arc)
        ),
        "references": format!("参考图由系统绑定：{}", compact_reference_ids(&segment.reference_image_ids)),
        "revision": "系统根据已验证片段规划生成审计字段；模型只负责最终可复制提示词文本"
    })
}
