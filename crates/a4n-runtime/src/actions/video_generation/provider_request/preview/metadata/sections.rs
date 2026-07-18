use super::super::super::super::*;

pub(super) fn preview_shot_sections() -> [(&'static str, Value); 12] {
    [
        (
            "self_contained_context",
            json!({
                "must_repeat_from_previous": ["预览占位：上一段必须带入的人物、空间、道具、光影和镜头信息"],
                "must_establish_in_segment": ["预览占位：本段独立生成时必须建立的剧情动作、场景锚点和情绪目标"],
                "must_preserve_throughout": ["预览占位：本段全程必须保持的人物身份、空间关系、光影方向和参考图约束"],
                "handoff_to_next": "预览占位：结尾要交给下一段继续的文字化状态",
                "omission_risks": ["预览占位：遗漏上下文会导致片段像独立短片、人物/空间/镜头断裂"]
            }),
        ),
        (
            "start_state",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全文字化起始状态，包括人物站位、姿态、朝向、场景锚点、机位和光影。"
            }),
        ),
        (
            "transition_in",
            json!({
                "from_previous_end": "预览占位：上一段结尾状态",
                "to_current_start": "预览占位：当前段起始状态",
                "cut_type": "预览占位：转场方式",
                "camera_transition": "预览占位：机位或视角变化",
                "first_beat_action": "预览占位：首个动作节拍"
            }),
        ),
        (
            "end_state",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全文字化结尾状态，作为下一段 start_state 的承接来源。"
            }),
        ),
        (
            "blocking_plan",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全人物站位、走位、空间轴线和首尾状态。"
            }),
        ),
        (
            "lighting_plan",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全主光方向、色温、阴影和首尾帧光影连续性。"
            }),
        ),
        (
            "emotion_action_beats",
            json!([{
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全情绪变化、可见动作和节拍。"
            }]),
        ),
        ("dialogue_beats", json!([])),
        (
            "cinematography_plan",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全景别、机位、镜头运动、轴线和焦段提示。"
            }),
        ),
        (
            "temporal_beats",
            json!([{
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全分秒段动作、镜头和连续性目标。"
            }]),
        ),
        (
            "reference_binding",
            json!({
                "bindings": [],
                "coverage_check": "请求预览占位；真实生成前需要覆盖所有参考图。"
            }),
        ),
        (
            "continuity_check",
            json!({
                "preview_placeholder": true,
                "blocking": "待补全",
                "lighting": "待补全",
                "emotion": "待补全",
                "references": "待补全",
                "revision": "真实生成前需要重新生成完整视频提示词。"
            }),
        ),
    ]
}
