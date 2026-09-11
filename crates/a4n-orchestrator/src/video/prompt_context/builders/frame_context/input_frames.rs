use a4n_error::AppError;
use serde_json::{Map, Value, json};

use a4n_domain::prompt_language::PromptOutputLanguage;

use super::segments::required_segment_number;

pub(in crate::video::prompt_context::builders) fn video_prompt_input_frames(
    frames: &Value,
    current_segment: &Value,
) -> Result<Value, AppError> {
    let segment_number = required_segment_number(current_segment, "current_segment")?;
    let mut input_frames = Map::new();
    if let Some(frame) = video_prompt_frame_by_number(frames, segment_number) {
        input_frames.insert("start_frame".to_string(), frame);
    }
    if let Some(frame) = video_prompt_frame_by_number(frames, segment_number + 1) {
        input_frames.insert("end_frame".to_string(), frame);
    }
    Ok(Value::Object(input_frames))
}

pub(in crate::video::prompt_context::builders) fn storyboard_frame_for_segment(
    frames: &Value,
    current_segment: &Value,
) -> Result<Value, AppError> {
    let segment_number = required_segment_number(current_segment, "current_segment")?;
    video_prompt_frame_by_number(frames, segment_number).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "分镜视频提示词请求构造失败：片段 #{} 缺少同号分镜提示词",
            segment_number
        ))
    })
}

pub(in crate::video::prompt_context::builders) fn add_input_frame_usage_constraint(
    constraints: &mut Map<String, Value>,
    input_frames: &Value,
    previous_completed_shot: Option<&Value>,
    prompt_output_language: PromptOutputLanguage,
) {
    let has_start_frame = input_frames.get("start_frame").is_some();
    let has_end_frame = input_frames.get("end_frame").is_some();
    let has_previous_completed_shot = previous_completed_shot
        .and_then(Value::as_object)
        .is_some_and(|object| !object.is_empty());
    let usage = match prompt_output_language {
        PromptOutputLanguage::ZhCn => json!({
            "opening_source": if has_start_frame {
                "input_frames.start_frame"
            } else if has_previous_completed_shot {
                "previous_completed_shot"
            } else {
                "current_segment"
            },
            "ending_source": if has_end_frame {
                "input_frames.end_frame"
            } else {
                "current_segment_end_state"
            },
            "opening_instruction": if has_start_frame {
                "开头画面以 input_frames.start_frame 提供的起点边界帧为准，描述该画面的人物身份、姿态、站位、道具、空间锚点、构图和光源方向；如果该边界帧已被后端绑定为参考图，正文按对应顺序写“参考图 N 为起点边界帧”，不要写 provider 图片槽位或内部字段名。"
            } else if has_previous_completed_shot {
                "开头画面必须承接 previous_completed_shot 的结尾状态，复述其人物站位、姿态、手中物、空间锚点、对方视线、光源方向或未完成动作；最终提示词直接写上一尾态的可见画面，不得写资源缺失、上传状态、内部槽位或纯文字兜底说明。"
            } else {
                "开头画面从 current_segment 的起点动作和场景锚点建立；最终提示词直接写可见画面状态，不得写资源缺失、上传状态、内部槽位或纯文字兜底说明。"
            },
            "ending_instruction": if has_end_frame {
                "结尾状态以 input_frames.end_frame 提供的终点边界帧为目标，在最后一个镜头描述如何稳定到该画面的人物姿态、位置、道具和光影；如果该边界帧已被后端绑定为参考图，正文按对应顺序写“参考图 N 为终点边界帧”，不要写 provider 图片槽位或内部字段名。"
            } else {
                "结尾状态由 current_segment.emotional_arc.end、continuity_handoff 和本段动作结果确定；最终提示词在最后一个镜头直接写可见姿态、表情、位置、道具和光影。"
            },
            "image_notes_instruction": if has_start_frame || has_end_frame {
                "主体定义与素材绑定章节只列 input_frames 中存在的边界帧参考图和实际选择的素材参考图；每一项都说明画面约束，并按后端绑定顺序使用“参考图 N”。"
            } else if has_previous_completed_shot {
                "主体定义与素材绑定章节写上一段尾态如何作为本段起始镜头，以及 reference_asset_summaries 中实际选择的参考图用途；不要报告图片槽位或资源状态。"
            } else {
                "主体定义与素材绑定章节写本段开场画面来源和实际参考图用途；不要报告图片槽位或资源状态。"
            },
            "upload_status_rule": "最终自然语言只能描述实际画面、动作、参考图用途和连续性；禁止报告资源是否存在、是否上传、是否缺失、内部槽位状态或纯文字兜底来源。"
        }),
        PromptOutputLanguage::En => json!({
            "opening_source": if has_start_frame {
                "input_frames.start_frame"
            } else if has_previous_completed_shot {
                "previous_completed_shot"
            } else {
                "current_segment"
            },
            "ending_source": if has_end_frame {
                "input_frames.end_frame"
            } else {
                "current_segment_end_state"
            },
            "opening_instruction": if has_start_frame {
                "Start from the start boundary frame represented by input_frames.start_frame. Describe visible character identity, pose, blocking, props, spatial anchors, composition, and light direction. If that boundary frame is bound by the backend as a reference image, refer to it by its backend order such as Reference Image N, not by a provider image slot or internal field name."
            } else if has_previous_completed_shot {
                "Start from previous_completed_shot's ending state. Restate its character blocking, pose, held props, spatial anchors, counterpart gaze, light direction, or unfinished action. Write the visible previous ending state directly; do not report missing resources, upload status, internal slots, or text-only fallback sources."
            } else {
                "Establish the opening from current_segment's initial action and spatial anchors. Write the visible screen state directly; do not report missing resources, upload status, internal slots, or text-only fallback sources."
            },
            "ending_instruction": if has_end_frame {
                "End toward the end boundary frame represented by input_frames.end_frame. In the final shot, describe how the image settles into that target pose, position, prop state, and lighting. If that boundary frame is bound by the backend as a reference image, refer to it by its backend order such as Reference Image N, not by a provider image slot or internal field name."
            } else {
                "Derive the ending from current_segment.emotional_arc.end, continuity_handoff, and the visible result of this segment's action. Write the final shot as concrete pose, expression, position, prop state, and lighting."
            },
            "image_notes_instruction": if has_start_frame || has_end_frame {
                "The Subject And Asset Binding section must list only boundary-frame reference images present in input_frames and selected material reference images. Describe the visual constraint for each item and use the backend reference order such as Reference Image N."
            } else if has_previous_completed_shot {
                "The Subject And Asset Binding section should state how the previous segment ending becomes this segment's opening shot and describe any selected reference-image purpose; never report image-slot availability or asset status."
            } else {
                "The Subject And Asset Binding section should describe the opening visual source and any selected reference-image purpose; never report image-slot availability or asset status."
            },
            "upload_status_rule": "Final natural-language prompts may only describe visible imagery, actions, reference-image purpose, and continuity; never report resource availability, upload state, missing assets, internal slots, or text-only fallback sources."
        }),
    };
    constraints.insert("input_frame_usage".to_string(), usage);
}

fn video_prompt_frame_by_number(frames: &Value, frame_number: i64) -> Option<Value> {
    frames
        .get("frames")?
        .as_array()?
        .iter()
        .find(|frame| {
            frame
                .get("frame_number")
                .and_then(Value::as_i64)
                .is_some_and(|number| number == frame_number)
        })
        .cloned()
}
