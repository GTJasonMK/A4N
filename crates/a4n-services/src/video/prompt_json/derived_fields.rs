use super::*;

pub(super) fn video_prompt_shot_from_input(
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
    let repeat = if segment.segment_number == 1 {
        json!([])
    } else {
        json!(["承接上一片段尾态的人物身份、服装、空间锚点、道具状态、光源方向和镜头轴线"])
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
                    .expect("validated segment emotional_arc must be summarizable")
            )
        ],
        "must_preserve_throughout": [
            "人物身份、服装、固定参照物、左右/前后关系、主要道具和光源方向必须连续",
            format!("参考图约束：{}", compact_reference_ids(&segment.reference_image_ids))
        ],
        "handoff_to_next": segment_end_summary(segment),
        "omission_risks": [
            "省略分段规划中的空间锚点会造成段间站位跳变",
            "省略情绪动作目标会削弱片段结尾承接"
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
        "from_previous_end": if segment.segment_number == 1 {
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
    segment
        .performance_plan
        .get("action_script")
        .expect("validated segment performance_plan must contain action_script")
        .clone()
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
    segment
        .performance_plan
        .get("time_blocks")
        .expect("validated segment performance_plan must contain time_blocks")
        .clone()
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
                .expect("validated segment emotional_arc must be summarizable")
        ),
        "references": format!("参考图由系统绑定：{}", compact_reference_ids(&segment.reference_image_ids)),
        "revision": "系统根据已验证片段规划生成审计字段；模型只负责最终可复制提示词文本"
    })
}

pub(super) fn derive_chat_video(segment: &VideoSegmentPlan) -> Value {
    let input_images = segment
        .reference_image_ids
        .iter()
        .enumerate()
        .map(|(index, image_id)| {
            json!({
                "image_id": image_id,
                "upload_order": index + 1,
                "role": "reference",
                "usage": "用于当前片段的人物、场景、道具或风格一致性约束"
            })
        })
        .collect::<Vec<_>>();
    json!({
        "source_span_ids": segment.source_span_ids,
        "timeline": [{
            "time": format!("0-{}秒", segment.duration_secs),
            "action": performance_summary(segment)
        }],
        "continuity": segment_end_summary(segment),
        "input_images": input_images
    })
}

fn character_summary(segment: &VideoSegmentPlan) -> String {
    first_structured_text_field(&segment.emotion_action_goals, "character")
        .expect("validated segment emotion_action_goals must contain character")
}

fn performance_summary(segment: &VideoSegmentPlan) -> String {
    let parts = [
        structured_text_field(&segment.performance_plan, "movement_path"),
        structured_text_field(&segment.performance_plan, "body_performance"),
        structured_text_field(&segment.performance_plan, "gaze_and_hands"),
        structured_text_field(&segment.performance_plan, "pause_or_reaction"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    assert!(
        !parts.is_empty(),
        "validated segment performance_plan must contain performance summary fields"
    );
    parts.join("；")
}

fn action_goal_summary(segment: &VideoSegmentPlan) -> String {
    structured_action_summary(&segment.emotion_action_goals)
        .expect("validated segment emotion_action_goals must be summarizable")
}

fn first_action_goal(segment: &VideoSegmentPlan) -> String {
    first_structured_action_summary(&segment.emotion_action_goals)
        .expect("validated segment emotion_action_goals must contain a first action")
}

fn segment_end_summary(segment: &VideoSegmentPlan) -> String {
    segment
        .emotional_arc
        .get("end")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .expect("validated segment emotional_arc must contain end")
}

fn emotional_end_summary(segment: &VideoSegmentPlan) -> String {
    segment
        .emotional_arc
        .get("end")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .expect("validated segment emotional_arc must contain end")
}

fn compact_reference_ids(ids: &[i64]) -> String {
    if ids.is_empty() {
        return "参考一致性由文字约束承担".to_string();
    }
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn structured_text_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn first_structured_text_field(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .find_map(|item| structured_text_field(item, key)),
        Value::Object(_) => structured_text_field(value, key),
        _ => None,
    }
}

fn structured_action_summary(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => {
            let parts = items
                .iter()
                .filter_map(structured_action_item_summary)
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("；"))
        }
        _ => structured_action_item_summary(value),
    }
}

fn first_structured_action_summary(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => items.iter().find_map(structured_action_item_summary),
        _ => structured_action_item_summary(value),
    }
}

fn structured_action_item_summary(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty_text(text),
        Value::Object(_) => {
            let parts = [
                structured_text_field(value, "character"),
                structured_text_field(value, "emotion"),
                structured_text_field(value, "visible_action"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("："))
        }
        _ => None,
    }
}

fn structured_arc_summary(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty_text(text),
        Value::Object(_) => {
            let parts = ["start", "turn", "end", "function"]
                .into_iter()
                .filter_map(|key| structured_text_field(value, key))
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join(" -> "))
        }
        _ => None,
    }
}

fn video_intent_summary(value: &Value) -> String {
    let parts = ["hook", "audience_effect", "rhythm", "visual_priority"]
        .into_iter()
        .filter_map(|key| structured_text_field(value, key))
        .collect::<Vec<_>>();
    assert!(
        !parts.is_empty(),
        "validated segment video_intent must contain summary fields"
    );
    parts.join("；")
}

fn material_coverage_summary(value: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(required) = string_array_field(value, "required_entities") {
        parts.push(format!("需要稳定：{required}"));
    }
    if let Some(missing) = string_array_field(value, "missing_or_weak_references") {
        parts.push(format!("弱覆盖：{missing}"));
    }
    if let Some(risk) = structured_text_field(value, "risk_note") {
        parts.push(format!("风险：{risk}"));
    }
    if parts.is_empty() {
        "按片段规划的素材覆盖要求保持一致".to_string()
    } else {
        parts.join("；")
    }
}

fn string_array_field(value: &Value, key: &str) -> Option<String> {
    let items = value.get(key)?.as_array()?;
    let text = items
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    (!text.is_empty()).then(|| text.join("、"))
}

fn non_empty_text(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
