use super::super::super::super::*;

pub(super) fn preview_placeholder_shot(prompt: &chapter_video_prompt::Model) -> Value {
    json!({
        "title": prompt.title.as_deref().unwrap_or("请求预览占位镜头"),
        "scene_summary": prompt.scene_summary.as_deref().unwrap_or("请求预览占位剧情摘要；真实生成前需要重新生成完整视频提示词。"),
        "blocking_plan": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全人物站位、走位、空间轴线和首尾状态。"
        },
        "lighting_plan": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全主光方向、色温、阴影和首尾帧光影连续性。"
        },
        "emotion_action_beats": [{
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全情绪变化、可见动作和节拍。"
        }],
        "dialogue_beats": [],
        "cinematography_plan": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全景别、机位、镜头运动、轴线和焦段提示。"
        },
        "temporal_beats": [{
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全分秒段动作、镜头和连续性目标。"
        }],
        "reference_binding": {
            "bindings": [],
            "coverage_check": "请求预览占位；真实生成前需要覆盖所有参考图。"
        },
        "continuity_check": {
            "preview_placeholder": true,
            "blocking": "待补全",
            "lighting": "待补全",
            "emotion": "待补全",
            "references": "待补全",
            "revision": "真实生成前需要重新生成完整视频提示词。"
        }
    })
}

pub(super) fn preview_placeholder_frame(
    prompt: &chapter_video_prompt::Model,
    label: &str,
) -> Value {
    json!({
        "title": format!("{label}（请求预览占位）"),
        "scene_summary": prompt.scene_summary.as_deref().unwrap_or("请求预览占位帧摘要；真实生成前需要重新生成完整帧提示词。"),
        "frame_prompt": format!(
            "【请求预览占位】{label}结构化帧提示缺失。真实生成前需要补全静态画面、人物站位、构图、光影、服装道具和可承接的结尾状态。"
        ),
        "blocking_state": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全人物、位置、姿态、道具状态和画面轴线。"
        },
        "lighting_state": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全主光方向、阴影、色温和环境光。"
        }
    })
}
