use a4n_core::error::AppError;
use serde_json::{Value, json};

use crate::video::prompt_generation::{
    expected_video_unit_count, storyboard_expected_segment_numbers, video_units_for_segment_plan,
};
use crate::video::prompt_json::VideoSegmentPlan;

use super::source::placeholder_segments;

pub(in crate::video::prompt_preview) fn placeholder_boundary_frame_context_for_video_prompt(
    segments: &[VideoSegmentPlan],
) -> Result<Value, AppError> {
    let segments = if segments.is_empty() {
        placeholder_segments()
    } else {
        segments.to_vec()
    };
    let duration_secs = segments[0].duration_secs;
    let units = video_units_for_segment_plan(&segments, duration_secs)?;
    let frame_count = units.len() + 1;
    let frames = (1..=frame_count)
        .map(|index| {
            let frame_number = i32::try_from(index).map_err(|_| {
                AppError::InvalidArgument(format!("边界帧占位编号超过 i32 范围：{index}"))
            })?;
            let segment_numbers = placeholder_boundary_segment_numbers(&units, index);
            let segment_number = segment_numbers.first().copied().ok_or_else(|| {
                AppError::InvalidArgument("边界帧占位缺少视频执行单元来源片段".to_string())
            })?;
            let segment = segments
                .iter()
                .find(|segment| segment.segment_number == segment_number)
                .ok_or_else(|| AppError::InvalidArgument(format!("边界帧占位找不到源片段 {segment_number}")))?;
            Ok(json!({
                "frame_number": frame_number,
                "title": format!("占位边界帧 #{}", frame_number),
                "scene_summary": "占位：真实生成时这里会替换为边界帧提示词输出的静态画面摘要。",
                "storyboard_panels": [],
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
                "segment_numbers": segment_numbers
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "preview_placeholder": true,
        "placeholder_source": "chapter_video_frame_prompts",
        "note": "真实生成时这里会替换为完整边界帧提示词结果；该占位仅用于展示后续视频提示词请求结构。",
        "frames": frames
    }))
}

fn placeholder_boundary_segment_numbers(
    units: &[crate::video::prompt_generation::VideoUnitPlan],
    frame_index: usize,
) -> Vec<i32> {
    debug_assert!(!units.is_empty());
    let frame_count = units.len() + 1;
    let clamped = frame_index.clamp(1, frame_count);
    let mut segment_numbers = Vec::new();
    if clamped > 1 {
        segment_numbers.extend(units[clamped - 2].source_segment_numbers());
    }
    if clamped <= units.len() {
        segment_numbers.extend(units[clamped - 1].source_segment_numbers());
    }
    segment_numbers.sort_unstable();
    segment_numbers.dedup();
    segment_numbers
}

pub(in crate::video::prompt_preview) fn placeholder_first_frame_prompts(
    segments: &[VideoSegmentPlan],
) -> Value {
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
                "storyboard_panels": [],
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

pub(in crate::video::prompt_preview) fn placeholder_storyboard_prompts(
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
            let frame_number = i32::try_from(index).map_err(|_| {
                AppError::InvalidArgument(format!("分镜板占位编号超过 i32 范围：{index}"))
            })?;
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
                "动作桥"
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
