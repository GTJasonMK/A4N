use a4n_core::config::app_config::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use a4n_core::error::AppError;
use a4n_engine::llm::provider::{ChatRequest, MessageRole};
use serde_json::{Value, json};

use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_context::preview_context_aware_chat_request;
use crate::video::prompt_generation::{
    expected_video_unit_count, storyboard_expected_segment_numbers,
};

use super::super::prompt_json::VideoSegmentPlan;
use super::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};

pub(super) fn step_from_request(
    key: &str,
    title: &str,
    prompt_name: &str,
    mut request: ChatRequest,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let output_token_reserve = request.max_tokens.unwrap_or(DEFAULT_LLM_MAX_OUTPUT_TOKENS);
    let context_preview = preview_context_aware_chat_request(
        &mut request,
        DEFAULT_LLM_MAX_CONTEXT_TOKENS,
        output_token_reserve,
    )?;
    let system_prompt = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::System))
        .map(|message| message.content.clone())
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "视频提示词预览步骤 {prompt_name} 缺少 system message"
            ))
        })?;
    let user_prompt = request
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::User))
        .map(|message| message.content.clone())
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "视频提示词预览步骤 {prompt_name} 缺少 user message"
            ))
        })?;
    let total_chars = system_prompt.chars().count() + user_prompt.chars().count();
    let char_counts = json!({
        "system_chars": system_prompt.chars().count(),
        "user_chars": user_prompt.chars().count(),
        "total_prompt_chars": total_chars,
        "context_stable_prefix_tokens": context_preview.as_ref().map(|preview| preview.stable_prefix_tokens),
        "context_dynamic_delta_tokens": context_preview.as_ref().map(|preview| preview.dynamic_delta_tokens),
        "context_available_input_tokens": context_preview.as_ref().map(|preview| preview.available_input_tokens)
    });
    let messages = serde_json::to_value(&request.messages)?;
    let request = context_aware_chat_request_trace_json(&request)?;
    Ok(VideoPromptRequestPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: prompt_name.to_string(),
        state: "ready".to_string(),
        note: None,
        system_prompt: Some(system_prompt),
        user_prompt: Some(user_prompt),
        messages: Some(messages),
        request: Some(request),
        context_manifest: context_preview
            .as_ref()
            .map(|preview| preview.manifest_json.clone()),
        context_plan: context_preview
            .as_ref()
            .map(|preview| preview.plan_json.clone()),
        context_pack_refs: context_preview
            .as_ref()
            .map(|preview| preview.pack_refs_json.clone()),
        total_chars,
        char_counts: Some(char_counts),
    })
}

pub(super) fn placeholder_step_from_request(
    key: &str,
    title: &str,
    prompt_name: &str,
    request: ChatRequest,
    note: &str,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let mut step = step_from_request(key, title, prompt_name, request)?;
    step.state = "preview_placeholder".to_string();
    step.note = Some(note.to_string());
    Ok(step)
}

pub(super) fn reused_step(
    key: &str,
    title: &str,
    prompt_name: &str,
    note: &str,
) -> VideoPromptRequestPreviewStep {
    VideoPromptRequestPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: prompt_name.to_string(),
        state: "reused".to_string(),
        note: Some(note.to_string()),
        system_prompt: None,
        user_prompt: None,
        messages: None,
        request: None,
        context_manifest: None,
        context_plan: None,
        context_pack_refs: None,
        total_chars: 0,
        char_counts: None,
    }
}

pub(super) fn placeholder_segments() -> Vec<VideoSegmentPlan> {
    vec![VideoSegmentPlan {
        segment_number: 1,
        analysis_shot_id: 1,
        segment_index_in_shot: 1,
        shot_segment_count: 1,
        covered_beat_ids: vec![1],
        title: "占位片段标题".to_string(),
        scene_summary: "占位：这里会替换为真实片段剧情摘要。".to_string(),
        blocking_summary: "占位：这里会替换为真实人物站位、移动目标和空间关系。".to_string(),
        spatial_anchors: json!([
            {
                "anchor": "占位固定参考物",
                "usage": "占位：约束人物位置和移动路径。"
            }
        ]),
        emotion_action_goals: json!([
            {
                "character": "占位人物",
                "emotion": "占位情绪",
                "visible_action": "占位可见动作"
            }
        ]),
        video_intent: json!({
            "hook": "占位：这里会替换为真实片段钩子。",
            "audience_effect": "占位：这里会替换为观众感受目标。",
            "rhythm": "占位节奏",
            "visual_priority": "占位：这里会替换为最重要的可见主体或动作。"
        }),
        emotional_arc: json!({
            "start": "占位起始情绪",
            "turn": "占位情绪转折",
            "end": "占位结尾情绪",
            "function": "占位段落功能"
        }),
        performance_plan: json!({
            "movement_path": "占位：这里会替换为真实起点、移动方向、终点和空间锚点。",
            "body_performance": "占位：这里会替换为身体重心、姿态变化、步幅或停顿。",
            "gaze_and_hands": "占位：这里会替换为视线落点、手部动作和道具状态。",
            "pause_or_reaction": "占位：这里会替换为关键停顿、反应或情绪余韵。",
            "camera_relationship": "占位：这里会替换为机位、景别、镜头运动与表演关系。",
            "time_blocks": [
                {
                    "start_sec": 0,
                    "end_sec": 6,
                    "camera": "占位镜头关系",
                    "subject_action": "占位主体动作",
                    "continuity_target": "占位可续接尾态"
                }
            ],
            "action_script": [
                {
                    "character": "占位人物",
                    "body": "占位身体变化",
                    "movement": "占位移动方向",
                    "hands": "占位手部动作",
                    "gaze": "占位视线变化",
                    "timing": "全段"
                }
            ]
        }),
        material_coverage: json!({
            "required_entities": ["占位人物", "占位场景"],
            "missing_or_weak_references": ["占位人物", "占位场景"],
            "risk_note": "占位：真实生成时这里会记录素材覆盖风险。"
        }),
        source_span_ids: vec!["ch1:s1:f1".to_string()],
        duration_secs: 6,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
    }]
}

pub(super) fn placeholder_extracted_beats(duration_secs: i32) -> Value {
    json!([
        {
            "beat_id": 1,
            "beat_type": "movement",
            "natural_seconds": duration_secs,
            "required": true,
            "visible_action": "占位主要动作",
            "source_span_ids": ["ch1:s1:f1"],
            "source_texts": ["占位：真实生成时这里会替换为当前正文窗口的原文片段。"]
        }
    ])
}

pub(super) fn first_cinematic_shot_for_preview(shot_analysis: &Value) -> Result<Value, AppError> {
    shot_analysis
        .get("cinematic_shots")
        .and_then(Value::as_array)
        .and_then(|shots| shots.first())
        .cloned()
        .ok_or_else(|| {
            AppError::InvalidArgument("视频提示词预览缺少 current_shot 上下文".to_string())
        })
}

pub(super) fn placeholder_shot_analysis(duration_secs: i32) -> Value {
    json!({
        "preview_placeholder": true,
        "placeholder_source": "chapter_video_shot_analysis",
        "note": "真实生成时这里会替换为上一步镜头语言分析模型输出。",
        "planning_quality": {
            "total_estimated_seconds": duration_secs,
            "total_required_segments": 1,
            "risk_flags": []
        },
        "cinematic_shots": [
            {
                "shot_id": 1,
                "title": "占位镜头",
                "source_span_ids": ["ch1:s1:f1"],
                "source_span_range": {"start": "ch1:s1:f1", "end": "ch1:s1:f1"},
                "key_source_span_ids": ["ch1:s1:f1"],
                "scene_intent": "占位：这里会替换为真实镜头意图。",
                "visible_actions": [{"character": "占位人物", "action": "占位动作", "purpose": "占位情绪"}],
                "dialogue_or_speech_intent": {"has_dialogue": false, "speaker": null, "key_line_or_intent": "", "visible_performance": "占位表演"},
                "blocking": {"start": "占位起点", "movement": "占位走位", "end": "占位终点", "spatial_anchors": ["占位锚点"]},
                "camera_intent": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "稳定推进", "visual_priority": "占位主体"},
                "transition_in": "占位进入",
                "transition_out": "占位退出",
                "segment_duration_secs": duration_secs,
                "required_segment_count": 1,
                "start_state": {"blocking": "占位起点", "emotion": "占位情绪", "camera": "占位镜头"},
                "end_state": {"blocking": "占位终点", "emotion": "占位情绪", "camera": "占位镜头"},
                "action_beats": [
                    {
                        "beat_id": 1,
                        "beat_type": "movement",
                        "natural_seconds": duration_secs,
                        "required": true,
                        "visible_action": "占位主要动作",
                        "start_state": {"blocking": "占位起点", "emotion": "占位情绪", "camera": "占位镜头"},
                        "end_state": {"blocking": "占位终点", "emotion": "占位情绪", "camera": "占位镜头"},
                        "source_span_ids": ["ch1:s1:f1"]
                    }
                ],
                "action_load": "light",
                "continuity_handoff": {"must_preserve": ["占位连续性"], "handoff_state": "占位交接状态"},
                "estimated_natural_seconds": duration_secs,
                "risk_flags": []
            }
        ]
    })
}

pub(super) fn placeholder_segment_plan() -> Value {
    json!({
        "preview_placeholder": true,
        "placeholder_source": "chapter_video_segment_plan",
        "note": "真实生成时这里会替换为上一步视频片段规划模型输出。",
        "segments": placeholder_segments()
    })
}

pub(super) fn placeholder_boundary_frame_context_for_video_prompt(
    segments: &[VideoSegmentPlan],
) -> Value {
    let segments = if segments.is_empty() {
        placeholder_segments()
    } else {
        segments.to_vec()
    };
    let frame_count = segments.len() * 2;
    let frames = (1..=frame_count)
        .map(|index| {
            let frame_number = index as i32;
            let segment_index = ((index.saturating_sub(1)) / 2).min(segments.len() - 1);
            let segment = &segments[segment_index];
            json!({
                "frame_number": frame_number,
                "title": format!("占位边界帧 #{}", frame_number),
                "scene_summary": "占位：真实生成时这里会替换为边界帧提示词输出的静态画面摘要。",
                "frame_prompt": "占位 frame_prompt：真实请求会使用前序边界帧提示词模型输出，包含人物、场景、构图、光源、风格和参考图约束。",
                "blocking_state": {
                    "characters": [],
                    "props": [],
                    "spatial_anchors": [],
                    "continuity_note": "占位：真实边界帧会给出人物站位和空间连续性。"
                },
                "lighting_state": {
                    "key_light": "占位主光",
                    "direction": "占位方向",
                    "continuity_note": "占位：真实边界帧会给出光影连续性。"
                },
                "negative_prompt": "占位 negative_prompt：真实请求会使用前序边界帧提示词输出。",
                "duration_secs": segment.duration_secs,
                "aspect_ratio": &segment.aspect_ratio,
                "reference_image_ids": [],
                "segment_numbers": placeholder_boundary_segment_numbers(&segments, index)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "preview_placeholder": true,
        "placeholder_source": "chapter_video_frame_prompts",
        "note": "真实生成时这里会替换为完整边界帧提示词结果；该占位仅用于展示后续视频提示词请求结构。",
        "frames": frames
    })
}

fn placeholder_boundary_segment_numbers(
    segments: &[VideoSegmentPlan],
    frame_index: usize,
) -> Vec<i32> {
    debug_assert!(!segments.is_empty());
    let frame_count = segments.len() * 2;
    let clamped = frame_index.clamp(1, frame_count);
    let segment_index = ((clamped - 1) / 2).min(segments.len() - 1);
    vec![segments[segment_index].segment_number]
}

pub(super) fn placeholder_first_frame_prompts(segments: &[VideoSegmentPlan]) -> Value {
    let segments = if segments.is_empty() {
        placeholder_segments()
    } else {
        segments.to_vec()
    };
    let segment = &segments[0];
    json!({
        "preview_placeholder": true,
        "placeholder_source": "chapter_video_first_frame_prompts",
        "note": "真实生成时这里会替换为初始首帧提示词模型输出。",
        "frames": [
            {
                "frame_number": 1,
                "title": "占位初始首帧",
                "scene_summary": "占位：真实生成时这里会替换为首帧串联模式第 1 段的初始画面摘要。",
                "frame_prompt": "占位 frame_prompt：真实请求会使用前序初始首帧提示词模型输出，只描述第 1 段视频开头的一张静态画面。",
                "blocking_state": {
                    "characters": [],
                    "props": [],
                    "spatial_anchors": [],
                    "continuity_note": "占位：真实首帧会给出第 1 段起始站位。"
                },
                "lighting_state": {
                    "key_light": "占位主光",
                    "direction": "占位方向",
                    "continuity_note": "占位：真实首帧会给出起始光影。"
                },
                "negative_prompt": "占位 negative_prompt：真实请求会使用前序初始首帧提示词输出。",
                "duration_secs": segment.duration_secs,
                "aspect_ratio": &segment.aspect_ratio,
                "reference_image_ids": [],
                "segment_numbers": [segment.segment_number]
            }
        ]
    })
}

pub(super) fn placeholder_storyboard_prompts(
    segments: &[VideoSegmentPlan],
) -> Result<Value, AppError> {
    let segments = if segments.is_empty() {
        placeholder_segments()
    } else {
        segments.to_vec()
    };
    let expected_count = expected_video_unit_count(&segments);
    let frames = (1..=expected_count)
        .map(|index| {
            let frame_number = index as i32;
            let segment_numbers = storyboard_expected_segment_numbers(&segments, frame_number)?;
            let segment_number = segment_numbers.first().copied().ok_or_else(|| {
                AppError::InvalidArgument("分镜板占位缺少视频单元源片段".to_string())
            })?;
            let segment = segments
                .iter()
                .find(|segment| segment.segment_number == segment_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "分镜板占位找不到源片段 {segment_number}"
                    ))
                })?;
            let unit_kind = if segment_numbers.len() > 1 {
                "过渡"
            } else {
                "内容"
            };
            Ok(json!({
                "frame_number": frame_number,
                "title": format!("占位{unit_kind}分镜板 #{}", frame_number),
                "scene_summary": "占位：真实生成时这里会替换为当前视频单元多格分镜板的画面摘要。",
                "storyboard_panels": [
                    {
                        "panel_index": 1,
                        "moment": "起始状态",
                        "visual_goal": "建立人物、空间和情绪",
                        "composition": "清晰展示主体站位和空间锚点",
                        "continuity_note": "延续人物身份、服装和光源方向"
                    },
                    {
                        "panel_index": 2,
                        "moment": "结尾状态",
                        "visual_goal": "表现动作结果和可续接尾态",
                        "composition": "保持同一场景关系并突出反应",
                        "continuity_note": "承接上一格动作变化"
                    }
                ],
                "frame_prompt": "占位 frame_prompt：真实请求会使用前序分镜提示词模型输出，生成一张动态多格分镜板；第 1 格建立起始状态，第 2 格表现动作结果，必要时增加第 3-6 格表达核心动作、听者反应或转场。",
                "blocking_state": {
                    "characters": [],
                    "props": [],
                    "spatial_anchors": [],
                    "continuity_note": "占位：真实分镜板会给出多格连续姿态和空间关系。"
                },
                "lighting_state": {
                    "key_light": "占位主光",
                    "direction": "占位方向",
                    "continuity_note": "占位：真实分镜板会给出本段连续光影状态。"
                },
                "negative_prompt": "占位 negative_prompt：真实请求会使用前序分镜提示词输出。",
                "duration_secs": segment.duration_secs,
                "aspect_ratio": &segment.aspect_ratio,
                "reference_image_ids": [],
                "segment_numbers": segment_numbers
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "preview_placeholder": true,
        "placeholder_source": "chapter_video_storyboard_prompts",
        "note": "真实生成时这里会替换为完整多格分镜板提示词结果；该占位仅用于展示后续分镜视频提示词请求结构。",
        "frames": frames
    }))
}

pub(super) fn placeholder_video_prompt_draft(prompt_name: &str) -> Value {
    json!({
        "preview_placeholder": true,
        "placeholder_source": prompt_name,
        "note": "真实生成时这里会替换为上一条视频提示词初稿模型输出。",
        "shots": [
            {
                "title": "占位镜头标题",
                "scene_summary": "占位：这里会替换为真实视频提示词初稿。",
                "visual_prompt": "[主体定义与素材绑定]\n占位：真实请求会使用上一条模型输出的完整画面提示词。\n\n[片段内容以及前因后果]\n占位：这里会替换为真实片段上下文。\n\n[镜头分镜]\n镜头 1：占位起始镜头，交代主体、景别、空间锚点和起始动作。\n镜头 2：占位收束镜头，交代动作桥、反应和结尾状态。\n\n[画面风格描述]\n占位：这里会替换为真实风格描述。\n\n[连续性与结尾状态]\n占位：这里会替换为真实结尾状态。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、角色变形或空间跳变。",
                "motion_prompt": "[运镜与动作]\n占位：真实请求会使用上一条模型输出的完整动作与镜头提示词。\n\n[对白与表演]\n无对白。",
                "negative_prompt": "占位 negative_prompt：真实请求会使用上一条模型输出。",
                "dialogue_cues": []
            }
        ]
    })
}

pub(super) fn preview_boundary_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_boundary_frame_generation",
        "边界帧完整提示词与模型请求",
        steps,
    )
}

pub(super) fn preview_first_frame_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_first_frame_generation",
        "首帧完整提示词与模型请求",
        steps,
    )
}

pub(super) fn preview_chat_boundary_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_chat_boundary_frame_generation",
        "Chat 首尾帧完整提示词与模型请求",
        steps,
    )
}

pub(super) fn preview_storyboard_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_storyboard_generation",
        "分镜完整提示词与模型请求",
        steps,
    )
}

pub(super) fn preview_video_prompt_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_prompt_generation",
        "视频完整提示词与模型请求",
        steps,
    )
}

pub(super) fn preview_storyboard_video_prompt_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_storyboard_prompt_generation",
        "分镜视频完整提示词与模型请求",
        steps,
    )
}

pub(super) fn preview_chat_video_prompt_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_chat_video_prompt_generation",
        "Chat 视频完整提示词与模型请求",
        steps,
    )
}

fn preview_from_steps_named(
    workflow: &str,
    title: &str,
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    VideoPromptRequestPreview {
        workflow: workflow.to_string(),
        title: title.to_string(),
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_engine::llm::provider::ChatMessage;

    fn chat_request(messages: Vec<ChatMessage>) -> ChatRequest {
        ChatRequest::new(messages, Some(0.2), Some(512), None)
    }

    fn message(role: MessageRole, content: &str) -> ChatMessage {
        ChatMessage::new(role, content)
    }

    #[test]
    fn step_from_request_rejects_missing_user_message() {
        let err = step_from_request(
            "segment_plan",
            "视频片段规划",
            "chapter_video_segment_plan",
            chat_request(vec![message(MessageRole::System, "system prompt")]),
        )
        .expect_err("missing user message must be visible");

        assert!(err.to_string().contains("缺少 user message"));
    }

    #[test]
    fn step_from_request_preserves_system_and_user_prompts() {
        let step = step_from_request(
            "segment_plan",
            "视频片段规划",
            "chapter_video_segment_plan",
            chat_request(vec![
                message(MessageRole::System, "system prompt"),
                message(MessageRole::User, "user prompt"),
            ]),
        )
        .expect("valid request should be previewed");

        assert_eq!(step.system_prompt.as_deref(), Some("system prompt"));
        assert_eq!(step.user_prompt.as_deref(), Some("user prompt"));
        assert_eq!(
            step.total_chars,
            "system prompt".chars().count() + "user prompt".chars().count()
        );
    }

    #[test]
    fn step_from_request_uses_message_roles_instead_of_fixed_positions() {
        let step = step_from_request(
            "segment_plan",
            "视频片段规划",
            "chapter_video_segment_plan",
            chat_request(vec![
                message(MessageRole::User, "old user prompt"),
                message(MessageRole::Assistant, "assistant draft"),
                message(MessageRole::System, "system prompt"),
                message(MessageRole::User, "latest user prompt"),
            ]),
        )
        .expect("valid request should be previewed by role");

        assert_eq!(step.system_prompt.as_deref(), Some("system prompt"));
        assert_eq!(step.user_prompt.as_deref(), Some("latest user prompt"));
    }
}
