use std::collections::{HashMap, HashSet};

use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::request::pretty_json_with_root_key_order;
use serde_json::{Map, Value, json};

use a4n_core::error::AppError;

use crate::chapter::source_spans::{SourceSpan, source_span_prompt_values};
use crate::prompt_language::PromptOutputLanguage;
use crate::video::prompt_json::{VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS, sanitize_prompt_asset_text};

use super::final_video_prompt_public_context;

const VIDEO_BUILDER_RULES_TEMPLATE: &str = "chapter_video_builder_rules";
const VIDEO_USER_REQUEST_TEMPLATE: &str = "chapter_video_user_request";
const VIDEO_PAYLOAD_ROOT_KEY_ORDER: &[&str] = &[
    "task",
    "constraints",
    "output_contract",
    "chapter",
    "previous_steps",
    "current_segment",
    "reference_asset_summaries",
    "input_frames",
];

#[derive(Clone, Copy)]
enum VideoPromptContractMode {
    Api,
    Chat,
}

#[derive(Clone, Copy)]
enum VideoPromptContractPass {
    Draft,
    Review,
}

struct VideoPromptCommonConstraintInput<'a> {
    rules: &'a Map<String, Value>,
    mode: VideoPromptContractMode,
    pass: VideoPromptContractPass,
    style: &'a str,
    prompt_output_language: PromptOutputLanguage,
    duration_secs: i32,
    aspect_ratio: &'a str,
    reference_ids: &'a Value,
    draft_prompt_field: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub(in crate::video) struct VideoPlanningPromptContext<'a> {
    pub prompt_engine: &'a dyn PromptEngine,
    pub project_id: i64,
    pub chapter_summary: Option<&'a str>,
    pub chapter_analysis: Option<&'a Value>,
    pub project_context: Option<&'a Value>,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
}

#[derive(Clone, Copy)]
pub(in crate::video) struct FinalVideoPromptContext<'a> {
    pub prompt_engine: &'a dyn PromptEngine,
    pub project_id: i64,
    pub chapter_summary: Option<&'a str>,
    pub chapter_analysis: Option<&'a Value>,
    pub segment_plan: &'a Value,
    pub frame_prompts: &'a Value,
    pub current_segment: &'a Value,
    pub previous_completed_shot: Option<&'a Value>,
    pub available_assets: &'a Value,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
}

#[derive(Clone, Copy)]
pub(in crate::video) struct FramePromptContext<'a> {
    pub prompt_engine: &'a dyn PromptEngine,
    pub project_id: i64,
    pub chapter_summary: Option<&'a str>,
    pub chapter_analysis: Option<&'a Value>,
    pub segment_plan: &'a Value,
    pub available_assets: &'a Value,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
}

fn video_builder_rules(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
) -> Result<Map<String, Value>, AppError> {
    let rendered = prompt_engine.render(
        VIDEO_BUILDER_RULES_TEMPLATE,
        &HashMap::new(),
        Some(project_id),
    )?;
    serde_json::from_str::<Value>(&rendered)
        .map_err(|err| {
            AppError::ConfigError(format!(
                "视频 builder 规则模板必须渲染为合法 JSON 对象：{err}"
            ))
        })?
        .as_object()
        .cloned()
        .ok_or_else(|| {
            AppError::ConfigError("视频 builder 规则模板必须渲染为 JSON 对象".to_string())
        })
}

fn rule_value(rules: &Map<String, Value>, key: &str) -> Result<Value, AppError> {
    rules
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::ConfigError(format!("视频 builder 规则模板缺少必需字段：{key}")))
}

fn rule_text(rules: &Map<String, Value>, key: &str) -> Result<String, AppError> {
    rule_value(rules, key)?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::ConfigError(format!("视频 builder 规则模板字段必须是字符串：{key}"))
        })
}

fn insert_rule_value(
    constraints: &mut Map<String, Value>,
    constraints_key: &str,
    rules: &Map<String, Value>,
    rule_key: &str,
) -> Result<(), AppError> {
    constraints.insert(constraints_key.to_string(), rule_value(rules, rule_key)?);
    Ok(())
}

fn video_prompt_task(
    rules: &Map<String, Value>,
    name: &str,
    instruction_key: &str,
) -> Result<Value, AppError> {
    Ok(json!({
        "name": name,
        "instruction": rule_text(rules, instruction_key)?
    }))
}

fn final_video_prompt_output_contract(prompt_name: &str) -> Value {
    json!({
        "format": "json_object",
        "prompt_name": prompt_name,
        "required_root_field": "shots",
        "required_shot_count": 1,
        "required_shot_fields": [
            "title",
            "scene_summary",
            "visual_prompt",
            "motion_prompt",
            "negative_prompt",
            "dialogue_cues"
        ],
        "dialogue_cue_allowed_fields": VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS,
        "dialogue_cue_field_rule": "dialogue_cues 每一项只能包含 output_contract.dialogue_cue_allowed_fields 中列出的字段；不要输出额外字段。"
    })
}

fn frame_reference_slots(
    reference_ids: &Value,
    first_slot_role: Option<&str>,
) -> Result<Value, AppError> {
    let reference_ids = reference_ids.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频帧提示词 reference_ids 必须是数组".to_string())
    })?;
    if first_slot_role.is_some() && reference_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频帧提示词上一帧参考图已存在，但 reference_ids 没有提供对应的第 1 个参考槽位"
                .to_string(),
        ));
    }
    let mut seen_ids = HashSet::with_capacity(reference_ids.len());
    let mut slots = Vec::with_capacity(reference_ids.len());
    for (index, value) in reference_ids.iter().enumerate() {
        let image_id = value.as_i64().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频帧提示词 reference_ids 第 {} 项必须是图片 ID 整数",
                index + 1
            ))
        })?;
        if image_id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "视频帧提示词 reference_ids 第 {} 项必须是正数图片 ID",
                index + 1
            )));
        }
        if !seen_ids.insert(image_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频帧提示词 reference_ids 包含重复图片 ID：{image_id}"
            )));
        }
        let slot_index = index + 1;
        let role = if slot_index == 1 {
            first_slot_role.unwrap_or("material_reference")
        } else {
            "material_reference"
        };
        let purpose = if role == "previous_boundary_frame" {
            "上一帧实际画面连续性约束"
        } else {
            "人物、场景、道具或风格一致性约束"
        };
        slots.push(json!({
            "slot_index": slot_index,
            "slot_label": format!("参考图 {slot_index}"),
            "role": role,
            "purpose": purpose
        }));
    }
    Ok(Value::Array(slots))
}

fn frame_reference_slot_order_rule() -> &'static str {
    "frame_prompt 自然语言中的参考图编号必须由后端提供的 constraints.reference_slots 顺序决定：第一项写“参考图 1”，第二项写“参考图 2”。不要输出 reference_image_ids，不要把 available_assets 的候选序号、候选素材标签、字段名或内部编号当成参考图编号；frame_prompt 和 negative_prompt 正文不得出现 image_id、image id、图片 ID、参考图 ID、available_assets、manual_library_asset_* 或任何原始图片数字 ID。"
}

fn video_prompt_reference_slots(
    reference_ids: &Value,
    mode: VideoPromptContractMode,
) -> Result<Value, AppError> {
    let reference_ids = reference_ids.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 reference_ids 必须是数组".to_string())
    })?;
    let mut seen = HashSet::with_capacity(reference_ids.len());
    let mut slots = Vec::with_capacity(reference_ids.len());
    for (index, value) in reference_ids.iter().enumerate() {
        let image_id = value.as_i64().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 reference_ids 第 {} 项必须是图片 ID 整数",
                index + 1
            ))
        })?;
        if image_id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 reference_ids 第 {} 项必须是正数图片 ID",
                index + 1
            )));
        }
        if !seen.insert(image_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 reference_ids 包含重复图片 ID：{image_id}"
            )));
        }
        let slot_index = index + 1;
        let slot_label = match mode {
            VideoPromptContractMode::Api => format!("参考图 {slot_index}"),
            VideoPromptContractMode::Chat => format!("第 {slot_index} 张图片"),
        };
        slots.push(json!({
            "slot_index": slot_index,
            "slot_label": slot_label,
            "purpose": match mode {
                VideoPromptContractMode::Api => "人物、场景、道具、风格或边界帧连续性约束",
                VideoPromptContractMode::Chat => "外部 Chat 页面按该顺序上传和描述的输入图片",
            }
        }));
    }
    Ok(Value::Array(slots))
}

fn reference_asset_summaries(available_assets: &Value) -> Result<Value, AppError> {
    let items = available_assets.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 reference_asset_summaries 来源必须是数组".to_string())
    })?;
    Ok(Value::Array(
        items.iter().map(reference_asset_summary).collect(),
    ))
}

fn reference_asset_summary(asset: &Value) -> Value {
    let mut object = Map::new();
    for key in [
        "role",
        "label",
        "description",
        "prompt_excerpt",
        "selection_reason",
        "profile_kind",
    ] {
        if let Some(value) = sanitize_prompt_asset_text(asset.get(key)) {
            object.insert(key.to_string(), Value::String(value));
        }
    }
    for key in ["is_primary", "is_favorite"] {
        if let Some(value) = asset.get(key).and_then(Value::as_bool) {
            object.insert(key.to_string(), Value::Bool(value));
        }
    }
    Value::Object(object)
}

fn video_prompt_common_constraints(
    input: VideoPromptCommonConstraintInput<'_>,
) -> Result<Map<String, Value>, AppError> {
    let VideoPromptCommonConstraintInput {
        rules,
        mode,
        pass,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field,
    } = input;
    let mut constraints = Map::new();
    constraints.insert("style".to_string(), json!(style));
    constraints.insert(
        "prompt_output_language".to_string(),
        prompt_output_language.to_prompt_json(),
    );
    constraints.insert(
        "prompt_output_language_instruction".to_string(),
        json!(prompt_output_language.instruction()),
    );
    constraints.insert("selected_duration_secs".to_string(), json!(duration_secs));
    constraints.insert(
        "duration_rule".to_string(),
        json!(video_prompt_duration_rule(rules, mode, pass)?),
    );
    constraints.insert("aspect_ratio".to_string(), json!(aspect_ratio));
    constraints.insert(
        "style_preservation_rule".to_string(),
        rule_value(rules, "style_preservation_rule")?,
    );
    let reference_slots = video_prompt_reference_slots(reference_ids, mode)?;
    constraints.insert("reference_slots".to_string(), reference_slots);
    constraints.insert(
        "shot_number_rule".to_string(),
        json!(video_prompt_shot_number_rule(
            rules,
            pass,
            draft_prompt_field
        )?),
    );
    insert_rule_value(
        &mut constraints,
        "segment_directing_rule",
        rules,
        "segment_directing_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "material_coverage_rule",
        rules,
        "material_coverage_rule",
    )?;
    constraints.insert(
        "prompt_required_sections".to_string(),
        json!(prompt_output_language.visual_sections()),
    );
    insert_rule_value(
        &mut constraints,
        "prompt_text_section_rule",
        rules,
        "final_prompt_section_rule",
    )?;
    constraints.insert(
        "prompt_density_rule".to_string(),
        json!(prompt_output_language.density_rule()),
    );
    insert_rule_value(
        &mut constraints,
        "action_load_rule",
        rules,
        "action_load_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "time_budget_rule",
        rules,
        "time_budget_rule",
    )?;
    constraints.insert(
        "dialogue_rule".to_string(),
        json!(video_prompt_dialogue_rule(rules, mode, pass)?),
    );
    insert_rule_value(
        &mut constraints,
        "sound_design_rule",
        rules,
        "sound_design_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "image_input_semantics_rule",
        rules,
        "image_prompt_semantics_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "input_frame_rule",
        rules,
        "input_frame_rule",
    )?;
    constraints.insert(
        "temporal_beat_rule".to_string(),
        json!(video_prompt_temporal_rule(rules, mode)?),
    );
    insert_rule_value(
        &mut constraints,
        "safe_visual_rewrite_rule",
        rules,
        "safe_visual_rewrite_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "reference_display_label_rule",
        rules,
        "reference_display_label_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "negative_prompt_rule",
        rules,
        "negative_prompt_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "natural_language_no_internal_fields_rule",
        rules,
        "natural_language_no_internal_fields_rule",
    )?;
    if matches!(pass, VideoPromptContractPass::Review) {
        insert_rule_value(
            &mut constraints,
            "prompt_review_rule",
            rules,
            "prompt_review_rule",
        )?;
        insert_rule_value(
            &mut constraints,
            "review_rule",
            rules,
            "review_revision_rule",
        )?;
    }
    Ok(constraints)
}

fn video_prompt_duration_rule(
    rules: &Map<String, Value>,
    mode: VideoPromptContractMode,
    pass: VideoPromptContractPass,
) -> Result<String, AppError> {
    let key = match (mode, pass) {
        (VideoPromptContractMode::Api, VideoPromptContractPass::Draft) => "duration_rule_api_draft",
        (VideoPromptContractMode::Api, VideoPromptContractPass::Review) => {
            "duration_rule_api_review"
        }
        (VideoPromptContractMode::Chat, VideoPromptContractPass::Draft) => {
            "duration_rule_chat_draft"
        }
        (VideoPromptContractMode::Chat, VideoPromptContractPass::Review) => {
            "duration_rule_chat_review"
        }
    };
    rule_text(rules, key)
}

fn video_prompt_shot_number_rule(
    rules: &Map<String, Value>,
    pass: VideoPromptContractPass,
    draft_prompt_field: Option<&str>,
) -> Result<String, AppError> {
    match pass {
        VideoPromptContractPass::Draft => rule_text(rules, "shot_number_rule_draft"),
        VideoPromptContractPass::Review => {
            let draft_prompt_field = draft_prompt_field.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "视频提示词 review 请求构造失败：缺少 draft_prompt_field"
                ))
            })?;
            Ok(rule_text(rules, "shot_number_rule_review")?
                .replace("{draft_prompt_field}", draft_prompt_field))
        }
    }
}

fn video_prompt_dialogue_rule(
    rules: &Map<String, Value>,
    mode: VideoPromptContractMode,
    pass: VideoPromptContractPass,
) -> Result<String, AppError> {
    let key = match (mode, pass) {
        (VideoPromptContractMode::Api, VideoPromptContractPass::Draft) => "dialogue_rule_api_draft",
        (VideoPromptContractMode::Api, VideoPromptContractPass::Review) => {
            "dialogue_rule_api_review"
        }
        (VideoPromptContractMode::Chat, VideoPromptContractPass::Draft) => {
            "dialogue_rule_chat_draft"
        }
        (VideoPromptContractMode::Chat, VideoPromptContractPass::Review) => {
            "dialogue_rule_chat_review"
        }
    };
    rule_text(rules, key)
}

fn video_prompt_temporal_rule(
    rules: &Map<String, Value>,
    mode: VideoPromptContractMode,
) -> Result<String, AppError> {
    let key = match mode {
        VideoPromptContractMode::Api => "temporal_beat_rule_api",
        VideoPromptContractMode::Chat => "temporal_beat_rule_chat",
    };
    rule_text(rules, key)
}

fn video_prompt_input_frames(frames: &Value, current_segment: &Value) -> Result<Value, AppError> {
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

fn storyboard_frame_for_segment(
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

fn add_input_frame_usage_constraint(
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

fn segment_number(value: &Value) -> Option<i64> {
    value.get("segment_number").and_then(Value::as_i64)
}

fn required_segment_number(value: &Value, context: &str) -> Result<i64, AppError> {
    segment_number(value).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "视频提示词请求构造失败：{context} 缺少有效 segment_number"
        ))
    })
}

fn segment_plan_items(segment_plan: &Value) -> Vec<&Value> {
    segment_plan
        .get("segments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

fn focused_segment_plan(segment_plan: &Value, current_segment: &Value) -> Value {
    let items = segment_plan_items(segment_plan);
    let current_number = segment_number(current_segment);
    let current_index = current_number
        .and_then(|number| {
            items
                .iter()
                .position(|segment| segment_number(segment) == Some(number))
        })
        .or_else(|| items.iter().position(|segment| *segment == current_segment));

    let previous_segment = current_index
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| items.get(index))
        .cloned()
        .cloned();
    let next_segment = current_index
        .and_then(|index| items.get(index + 1))
        .cloned()
        .cloned();

    json!({
        "segment_count": items.len(),
        "current_segment": current_segment,
        "previous_segment": previous_segment,
        "next_segment": next_segment
    })
}

fn lightweight_chapter_context(
    chapter_summary: Option<&str>,
    chapter_analysis: Option<&Value>,
) -> Value {
    json!({
        "summary": chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
        "analysis": chapter_analysis_context(chapter_analysis)
    })
}

fn frame_prompt_context(segment_plan: &Value, current_frame: &Value) -> Value {
    let expected_segment_numbers = current_frame
        .get("expected_segment_numbers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .collect::<Vec<_>>();
    let segments = segment_plan_items(segment_plan);
    let relevant_segments = segments
        .into_iter()
        .filter(|segment| {
            segment_number(segment).is_some_and(|number| expected_segment_numbers.contains(&number))
        })
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "relevant_segments": relevant_segments,
        "previous_segment": current_frame.get("previous_segment").cloned().unwrap_or(Value::Null),
        "next_segment": current_frame.get("next_segment").cloned().unwrap_or(Value::Null),
        "previous_frame": current_frame.get("previous_frame").cloned().unwrap_or(Value::Null),
        "previous_frame_image": current_frame.get("previous_frame_image").cloned().unwrap_or(Value::Null)
    })
}

fn required_current_frame_field(
    current_frame: &Value,
    key: &str,
    expected: &str,
) -> Result<Value, AppError> {
    let value = current_frame.get(key).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "视频边界帧提示词请求构造失败：current_frame 缺少系统字段 {key}"
        ))
    })?;
    if value.is_null() {
        return Err(AppError::InvalidArgument(format!(
            "视频边界帧提示词请求构造失败：current_frame.{key} 不能为 null"
        )));
    }
    match expected {
        "integer" if !value.is_i64() && !value.is_u64() => Err(AppError::InvalidArgument(format!(
            "视频边界帧提示词请求构造失败：current_frame.{key} 必须是整数"
        ))),
        "array" if value.as_array().is_none_or(|items| items.is_empty()) => {
            Err(AppError::InvalidArgument(format!(
                "视频边界帧提示词请求构造失败：current_frame.{key} 必须是非空数组"
            )))
        }
        "string" if value.as_str().is_none_or(|text| text.trim().is_empty()) => {
            Err(AppError::InvalidArgument(format!(
                "视频边界帧提示词请求构造失败：current_frame.{key} 必须是非空字符串"
            )))
        }
        _ => Ok(value.clone()),
    }
}

pub(in crate::video) fn build_boundary_video_prompt_user(
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts: boundary_frames,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let input_frames = video_prompt_input_frames(boundary_frames, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Api,
        pass: VideoPromptContractPass::Draft,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: None,
    })?;
    add_input_frame_usage_constraint(
        &mut constraints,
        &input_frames,
        previous_completed_shot,
        prompt_output_language,
    );
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "boundary_continuity_rule_draft",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_prompt",
        "task_instruction_chapter_video_prompt",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_prompt")?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_frame_prompts": input_frames.clone(),
            "previous_completed_shot": previous_completed_shot
        },
        "current_segment": current_segment,
        "reference_asset_summaries": reference_asset_summaries,
        "input_frames": input_frames,
        "constraints": constraints,
        "output_contract": final_video_prompt_output_contract("chapter_video_prompt")
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_boundary_video_prompt_review_user(
    context: FinalVideoPromptContext<'_>,
    draft_prompt: &Value,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts: boundary_frames,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let input_frames = video_prompt_input_frames(boundary_frames, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Api,
        pass: VideoPromptContractPass::Review,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: Some("draft_chapter_video_prompt"),
    })?;
    add_input_frame_usage_constraint(
        &mut constraints,
        &input_frames,
        previous_completed_shot,
        prompt_output_language,
    );
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "boundary_continuity_rule_review",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_prompt_review",
        "task_instruction_chapter_video_prompt_review",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_prompt_review")?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_frame_prompts": input_frames.clone(),
            "previous_completed_shot": previous_completed_shot,
            "draft_chapter_video_prompt": draft_prompt
        },
        "current_segment": current_segment,
        "reference_asset_summaries": reference_asset_summaries,
        "input_frames": input_frames,
        "constraints": constraints,
        "output_contract": final_video_prompt_output_contract("chapter_video_prompt_review")
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_storyboard_video_prompt_user(
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts: storyboard_frames,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let storyboard_frame = storyboard_frame_for_segment(storyboard_frames, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Api,
        pass: VideoPromptContractPass::Draft,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: None,
    })?;
    insert_rule_value(
        &mut constraints,
        "storyboard_usage_rule",
        &rules,
        "storyboard_video_usage_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "storyboard_video_continuity_rule_draft",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_storyboard_video_prompt",
        "task_instruction_chapter_video_storyboard_video_prompt",
    )?;
    let user_instruction = rule_text(
        &rules,
        "user_instruction_chapter_video_storyboard_video_prompt",
    )?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_storyboard_prompts": storyboard_frame.clone(),
            "previous_completed_shot": previous_completed_shot
        },
        "current_segment": current_segment,
        "storyboard_frame": storyboard_frame,
        "reference_asset_summaries": reference_asset_summaries,
        "constraints": constraints,
        "output_contract": final_video_prompt_output_contract("chapter_video_storyboard_video_prompt")
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_storyboard_video_prompt_review_user(
    context: FinalVideoPromptContext<'_>,
    draft_prompt: &Value,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts: storyboard_frames,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let storyboard_frame = storyboard_frame_for_segment(storyboard_frames, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Api,
        pass: VideoPromptContractPass::Review,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: Some("draft_chapter_video_storyboard_video_prompt"),
    })?;
    insert_rule_value(
        &mut constraints,
        "storyboard_usage_rule",
        &rules,
        "storyboard_video_usage_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "storyboard_video_continuity_rule_review",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_storyboard_video_prompt_review",
        "task_instruction_chapter_video_storyboard_video_prompt_review",
    )?;
    let user_instruction = rule_text(
        &rules,
        "user_instruction_chapter_video_storyboard_video_prompt_review",
    )?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_storyboard_prompts": storyboard_frame.clone(),
            "previous_completed_shot": previous_completed_shot,
            "draft_chapter_video_storyboard_video_prompt": draft_prompt
        },
        "current_segment": current_segment,
        "storyboard_frame": storyboard_frame,
        "reference_asset_summaries": reference_asset_summaries,
        "constraints": constraints,
        "output_contract": final_video_prompt_output_contract("chapter_video_storyboard_video_prompt_review")
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_first_frame_video_prompt_user(
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts: first_frames,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let input_frames = video_prompt_input_frames(first_frames, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Api,
        pass: VideoPromptContractPass::Draft,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: None,
    })?;
    add_input_frame_usage_constraint(
        &mut constraints,
        &input_frames,
        previous_completed_shot,
        prompt_output_language,
    );
    insert_rule_value(
        &mut constraints,
        "serial_continuity_rule",
        &rules,
        "first_frame_serial_continuity_rule_draft",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_first_frame_video_prompt",
        "task_instruction_chapter_video_first_frame_video_prompt",
    )?;
    let user_instruction = rule_text(
        &rules,
        "user_instruction_chapter_video_first_frame_video_prompt",
    )?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_first_frame_prompts": input_frames.clone(),
            "previous_completed_shot": previous_completed_shot
        },
        "current_segment": current_segment,
        "reference_asset_summaries": reference_asset_summaries,
        "input_frames": input_frames,
        "constraints": constraints,
        "output_contract": final_video_prompt_output_contract("chapter_video_first_frame_video_prompt")
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_first_frame_video_prompt_review_user(
    context: FinalVideoPromptContext<'_>,
    draft_prompt: &Value,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts: first_frames,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let input_frames = video_prompt_input_frames(first_frames, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Api,
        pass: VideoPromptContractPass::Review,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: Some("draft_chapter_video_first_frame_video_prompt"),
    })?;
    add_input_frame_usage_constraint(
        &mut constraints,
        &input_frames,
        previous_completed_shot,
        prompt_output_language,
    );
    insert_rule_value(
        &mut constraints,
        "serial_continuity_rule",
        &rules,
        "first_frame_serial_continuity_rule_review",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_first_frame_video_prompt_review",
        "task_instruction_chapter_video_first_frame_video_prompt_review",
    )?;
    let user_instruction = rule_text(
        &rules,
        "user_instruction_chapter_video_first_frame_video_prompt_review",
    )?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_first_frame_prompts": input_frames.clone(),
            "previous_completed_shot": previous_completed_shot,
            "draft_chapter_video_first_frame_video_prompt": draft_prompt
        },
        "current_segment": current_segment,
        "reference_asset_summaries": reference_asset_summaries,
        "input_frames": input_frames,
        "constraints": constraints,
        "output_contract": final_video_prompt_output_contract("chapter_video_first_frame_video_prompt_review")
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_chat_video_prompt_user(
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let input_frames = video_prompt_input_frames(frame_prompts, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Chat,
        pass: VideoPromptContractPass::Draft,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: None,
    })?;
    add_input_frame_usage_constraint(
        &mut constraints,
        &input_frames,
        previous_completed_shot,
        prompt_output_language,
    );
    insert_rule_value(
        &mut constraints,
        "chat_prompt_scope",
        &rules,
        "chat_prompt_scope_rule",
    )?;
    constraints.insert(
        "required_sections".to_string(),
        json!(prompt_output_language.visual_sections()),
    );
    insert_rule_value(
        &mut constraints,
        "input_image_rule",
        &rules,
        "chat_input_image_rule_draft",
    )?;
    insert_rule_value(
        &mut constraints,
        "director_rule",
        &rules,
        "chat_director_rule",
    )?;
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "chat_continuity_rule_draft",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_chat_prompt",
        "task_instruction_chapter_video_chat_prompt",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_chat_prompt")?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_frame_prompts": input_frames.clone(),
            "previous_completed_shot": previous_completed_shot
        },
        "current_segment": current_segment,
        "reference_asset_summaries": reference_asset_summaries,
        "input_frames": input_frames,
        "constraints": constraints,
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_chat_prompt",
            "required_root_field": "shots",
            "required_shot_count": 1,
            "required_shot_fields": [
                "title",
                "scene_summary",
                "chat_prompt",
                "negative_prompt"
            ],
            "chat_prompt_required_sections": prompt_output_language.visual_sections()
        }
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_chat_video_prompt_review_user(
    context: FinalVideoPromptContext<'_>,
    draft_prompt: &Value,
) -> Result<String, AppError> {
    let FinalVideoPromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        frame_prompts,
        current_segment,
        previous_completed_shot,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let input_frames = video_prompt_input_frames(frame_prompts, current_segment)?;
    let mut constraints = video_prompt_common_constraints(VideoPromptCommonConstraintInput {
        rules: &rules,
        mode: VideoPromptContractMode::Chat,
        pass: VideoPromptContractPass::Review,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        draft_prompt_field: Some("draft_chapter_video_chat_prompt"),
    })?;
    add_input_frame_usage_constraint(
        &mut constraints,
        &input_frames,
        previous_completed_shot,
        prompt_output_language,
    );
    insert_rule_value(
        &mut constraints,
        "review_scope",
        &rules,
        "chat_review_scope_rule",
    )?;
    constraints.insert(
        "required_sections".to_string(),
        json!(prompt_output_language.visual_sections()),
    );
    insert_rule_value(
        &mut constraints,
        "input_image_rule",
        &rules,
        "chat_input_image_rule_review",
    )?;
    insert_rule_value(
        &mut constraints,
        "continuity_rule",
        &rules,
        "chat_continuity_rule_review",
    )?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_chat_prompt_review",
        "task_instruction_chapter_video_chat_prompt_review",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_chat_prompt_review")?;
    let reference_asset_summaries = reference_asset_summaries(available_assets)?;
    let payload = final_video_prompt_public_context(&json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": focused_segment_plan(segment_plan, current_segment),
            "focused_chapter_video_frame_prompts": input_frames.clone(),
            "previous_completed_shot": previous_completed_shot,
            "draft_chapter_video_chat_prompt": draft_prompt
        },
        "current_segment": current_segment,
        "reference_asset_summaries": reference_asset_summaries,
        "input_frames": input_frames,
        "constraints": constraints,
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_chat_prompt_review",
            "required_root_field": "shots",
            "required_shot_count": 1
        }
    }));
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_beat_extraction_user(
    context: VideoPlanningPromptContext<'_>,
    source_spans: &[SourceSpan],
    window_index: usize,
    window_count: usize,
) -> Result<String, AppError> {
    let rules = video_builder_rules(context.prompt_engine, context.project_id)?;
    let allowed_source_span_ids = source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<Vec<_>>();
    let task = video_prompt_task(
        &rules,
        "chapter_video_beat_extraction",
        "task_instruction_chapter_video_beat_extraction",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_beat_extraction")?;
    let payload = json!({
        "task": task,
        "chapter": {
            "source_spans": source_span_prompt_values(source_spans),
            "summary": context.chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
            "analysis": chapter_analysis_context(context.chapter_analysis)
        },
        "project_context": context.project_context,
        "window": {
            "index": window_index,
            "count": window_count,
            "source_span_start": source_spans.first().map(|span| span.id.as_str()),
            "source_span_end": source_spans.last().map(|span| span.id.as_str()),
            "allowed_source_span_ids": allowed_source_span_ids
        },
        "constraints": {
            "style": context.style,
            "selected_duration_secs": context.duration_secs,
            "aspect_ratio": context.aspect_ratio,
            "style_preservation_rule": rule_value(&rules, "style_preservation_rule")?,
            "time_budget_rule": rule_value(&rules, "time_budget_rule")?,
            "safe_visual_rewrite_rule": rule_value(&rules, "safe_visual_rewrite_rule")?,
            "source_span_rule": rule_value(&rules, "shot_analysis_source_span_rule")?,
            "beat_granularity_rule": rule_value(&rules, "shot_analysis_beat_granularity_rule")?,
            "beat_split_rule": rule_value(&rules, "shot_analysis_beat_split_rule")?,
            "beat_timing_rule": rule_value(&rules, "shot_analysis_beat_timing_rule")?,
            "action_beat_field_rule": rule_value(&rules, "shot_analysis_action_beat_field_rule")?
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_beat_extraction",
            "required_root_field": "beats",
            "required_beat_fields": [
                "natural_seconds",
                "required",
                "visible_action",
                "source_span_ids"
            ],
            "derived_beat_fields": [
                "beat_id",
                "beat_type"
            ],
            "beat_number_rule": "不要输出 beat_id、beat_type、start_state、end_state、description、notes 或 reason；后端会按数组顺序和 visible_action 推导。source_span_ids 只能从 window.allowed_source_span_ids 中选择。"
        }
    });
    render_strict_json_user_prompt(
        context.prompt_engine,
        context.project_id,
        &user_instruction,
        &payload,
    )
}

pub(in crate::video) fn build_shot_grouping_user(
    context: VideoPlanningPromptContext<'_>,
    extracted_beats: &Value,
) -> Result<String, AppError> {
    let rules = video_builder_rules(context.prompt_engine, context.project_id)?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_shot_analysis",
        "task_instruction_chapter_video_shot_analysis",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_shot_analysis")?;
    let payload = json!({
        "task": task,
        "chapter": {
            "summary": context.chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
            "analysis": chapter_analysis_context(context.chapter_analysis)
        },
        "project_context": context.project_context,
        "previous_steps": {
            "chapter_video_beat_extraction": {
                "beats": extracted_beats
            }
        },
        "constraints": {
            "style": context.style,
            "selected_duration_secs": context.duration_secs,
            "aspect_ratio": context.aspect_ratio,
            "style_preservation_rule": rule_value(&rules, "style_preservation_rule")?,
            "time_budget_rule": rule_value(&rules, "time_budget_rule")?,
            "safe_visual_rewrite_rule": rule_value(&rules, "safe_visual_rewrite_rule")?,
            "source_span_range_rule": rule_value(&rules, "shot_analysis_source_span_range_rule")?,
            "segment_semantics_rule": rule_value(&rules, "shot_analysis_segment_semantics_rule")?,
            "shot_duration_rule": rule_value(&rules, "shot_analysis_duration_rule")?
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_shot_analysis",
            "required_root_field": "cinematic_shots",
            "required_shot_fields": [
                "beat_ids",
                "title",
                "scene_intent",
                "visible_actions",
                "dialogue_or_speech_intent",
                "blocking",
                "camera_intent",
                "transition_in",
                "transition_out",
                "start_state",
                "end_state",
                "action_load",
                "continuity_handoff"
            ],
            "derived_shot_fields": [
                "shot_id",
                "source_span_ids",
                "source_span_range",
                "key_source_span_ids",
                "action_beats",
                "estimated_natural_seconds",
                "segment_duration_secs",
                "required_segment_count",
                "risk_flags",
                "planning_quality"
            ],
            "shot_number_rule": "不要输出 shot_id、source_span_ids、source_span_range、key_source_span_ids、action_beats、estimated_natural_seconds、segment_duration_secs、required_segment_count、risk_flags 或 planning_quality；后端会根据 beat_ids、动作节拍和用户选择时长推导。beat_ids 必须按 previous_steps.chapter_video_beat_extraction.beats[].beat_id 连续覆盖，不能遗漏、重复、倒序或跨镜头回跳。"
        }
    });
    render_strict_json_user_prompt(
        context.prompt_engine,
        context.project_id,
        &user_instruction,
        &payload,
    )
}

pub(in crate::video) fn build_segment_plan_for_shot_user(
    context: VideoPlanningPromptContext<'_>,
    shot_analysis: &Value,
    current_shot: &Value,
    source_spans: &[SourceSpan],
) -> Result<String, AppError> {
    let rules = video_builder_rules(context.prompt_engine, context.project_id)?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_segment_plan",
        "task_instruction_chapter_video_segment_plan",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_segment_plan")?;
    let payload = json!({
        "task": task,
        "chapter": {
            "source_spans": source_span_prompt_values(source_spans),
            "summary": context.chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
            "analysis": chapter_analysis_context(context.chapter_analysis)
        },
        "project_context": context.project_context,
        "previous_steps": {
            "chapter_video_shot_analysis": shot_analysis
        },
        "current_shot": current_shot,
        "constraints": {
            "style": context.style,
            "selected_duration_secs": context.duration_secs,
            "aspect_ratio": context.aspect_ratio,
            "style_preservation_rule": rule_value(&rules, "style_preservation_rule")?,
            "time_budget_rule": rule_value(&rules, "time_budget_rule")?,
            "safe_visual_rewrite_rule": rule_value(&rules, "safe_visual_rewrite_rule")?,
            "source_span_rule": rule_value(&rules, "segment_plan_source_span_rule")?,
            "shot_analysis_rule": rule_value(&rules, "segment_plan_shot_analysis_rule")?,
            "beat_mapping_rule": rule_value(&rules, "segment_plan_beat_mapping_rule")?,
            "shot_mapping_rule": rule_value(&rules, "segment_plan_shot_mapping_rule")?,
            "duration_selection_rule": rule_value(&rules, "segment_plan_duration_selection_rule")?
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_segment_plan",
            "required_root_field": "segments",
            "required_segment_fields": [
                "analysis_shot_id",
                "covered_beat_ids",
                "title",
                "scene_summary",
                "blocking_summary",
                "spatial_anchors",
                "emotion_action_goals",
                "video_intent",
                "emotional_arc",
                "performance_plan",
                "material_coverage"
            ],
            "derived_segment_fields": [
                "segment_number",
                "segment_index_in_shot",
                "shot_segment_count",
                "source_span_ids",
                "duration_secs",
                "aspect_ratio"
            ],
            "segment_number_rule": "只为 current_shot 生成 1 个或多个连续 segments；analysis_shot_id 必须等于 current_shot.shot_id；covered_beat_ids 使用 current_shot.action_beats[].beat_id 的局部编号。不要输出 segment_number、segment_index_in_shot、shot_segment_count、source_span_ids、duration_secs、aspect_ratio 或 reference_image_ids；后端会按数组顺序、镜头分析、用户选择时长和用户画幅推导结构字段。片段规划只做素材需求审计，不绑定具体参考图。",
            "performance_action_script_rule": "performance_plan.action_script 每个对象必须同时包含 character、body、movement、hands、gaze、timing 六个字段，且全部必须是非空字符串。hands 没有明显手部动作时也必须写可见静止状态或未接触道具状态，例如“手自然垂在身侧”“手未接触道具”“双手保持稳定”；禁止空字符串、null、对象、数组或省略字段。"
        }
    });
    render_strict_json_user_prompt(
        context.prompt_engine,
        context.project_id,
        &user_instruction,
        &payload,
    )
}

pub(in crate::video) fn build_single_frame_prompt_user(
    context: FramePromptContext<'_>,
    current_frame: &Value,
) -> Result<String, AppError> {
    let FramePromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let required_frame_number =
        required_current_frame_field(current_frame, "frame_number", "integer")?;
    let required_segment_numbers =
        required_current_frame_field(current_frame, "expected_segment_numbers", "array")?;
    let required_duration_secs =
        required_current_frame_field(current_frame, "required_duration_secs", "integer")?;
    let required_aspect_ratio =
        required_current_frame_field(current_frame, "required_aspect_ratio", "string")?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_frame_prompts",
        "task_instruction_chapter_video_frame_prompts",
    )?;
    let first_slot_role = current_frame
        .get("previous_frame_image")
        .filter(|value| !value.is_null())
        .map(|_| "previous_boundary_frame");
    let reference_slots = frame_reference_slots(reference_ids, first_slot_role)?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_frame_prompts")?;
    let payload = json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": frame_prompt_context(segment_plan, current_frame)
        },
        "current_frame": current_frame,
        "available_assets": available_assets,
        "constraints": {
            "style": style,
            "style_template_verbatim": style,
            "prompt_output_language": prompt_output_language.to_prompt_json(),
            "prompt_output_language_instruction": prompt_output_language.instruction(),
            "frame_prompt_required_sections": prompt_output_language.frame_sections(),
            "aspect_ratio": aspect_ratio,
            "reference_slots": reference_slots,
            "single_frame_rule": rule_value(&rules, "single_frame_rule")?,
            "style_preservation_rule": rule_value(&rules, "single_frame_style_preservation_rule")?,
            "frame_prompt_section_rule": rule_value(&rules, "frame_image_prompt_section_rule")?,
            "boundary_continuity_rule": rule_value(&rules, "frame_boundary_continuity_rule")?,
            "previous_frame_rule": rule_value(&rules, "single_frame_previous_frame_rule")?,
            "reference_display_label_rule": rule_value(&rules, "reference_display_label_rule")?,
            "reference_slot_order_rule": frame_reference_slot_order_rule(),
            "segment_directing_rule": rule_value(&rules, "single_frame_segment_directing_rule")?,
            "material_coverage_rule": rule_value(&rules, "material_coverage_rule")?,
            "prompt_density_rule": prompt_output_language.density_rule()
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_frame_prompts",
            "required_root_field": "frames",
            "required_frame_count": 1,
            "required_frame_number": required_frame_number,
            "required_segment_numbers": required_segment_numbers,
            "required_duration_secs": required_duration_secs,
            "required_aspect_ratio": required_aspect_ratio
        }
    });
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_storyboard_prompt_user(
    context: FramePromptContext<'_>,
    current_frame: &Value,
) -> Result<String, AppError> {
    let FramePromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let required_frame_number =
        required_current_frame_field(current_frame, "frame_number", "integer")?;
    let required_segment_numbers =
        required_current_frame_field(current_frame, "expected_segment_numbers", "array")?;
    let required_duration_secs =
        required_current_frame_field(current_frame, "required_duration_secs", "integer")?;
    let required_aspect_ratio =
        required_current_frame_field(current_frame, "required_aspect_ratio", "string")?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_storyboard_prompts",
        "task_instruction_chapter_video_storyboard_prompts",
    )?;
    let reference_slots = frame_reference_slots(reference_ids, None)?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_storyboard_prompts")?;
    let payload = json!({
        "task": task,
        "chapter": lightweight_chapter_context(chapter_summary, chapter_analysis),
        "previous_steps": {
            "focused_chapter_video_segment_plan": frame_prompt_context(segment_plan, current_frame)
        },
        "current_storyboard_frame": current_frame,
        "available_assets": available_assets,
        "constraints": {
            "style": style,
            "style_template_verbatim": style,
            "prompt_output_language": prompt_output_language.to_prompt_json(),
            "prompt_output_language_instruction": prompt_output_language.instruction(),
            "frame_prompt_required_sections": prompt_output_language.frame_sections(),
            "aspect_ratio": aspect_ratio,
            "reference_slots": reference_slots,
            "storyboard_frame_rule": rule_value(&rules, "storyboard_frame_rule")?,
            "storyboard_panel_count_rule": rule_value(&rules, "storyboard_panel_count_rule")?,
            "style_preservation_rule": rule_value(&rules, "style_preservation_rule")?,
            "reference_display_label_rule": rule_value(&rules, "reference_display_label_rule")?,
            "reference_slot_order_rule": frame_reference_slot_order_rule(),
            "segment_directing_rule": rule_value(&rules, "storyboard_segment_directing_rule")?,
            "material_coverage_rule": rule_value(&rules, "material_coverage_rule")?,
            "prompt_density_rule": prompt_output_language.density_rule()
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_storyboard_prompts",
            "required_root_field": "frames",
            "required_frame_count": 1,
            "required_frame_number": required_frame_number,
            "required_segment_numbers": required_segment_numbers,
            "required_duration_secs": required_duration_secs,
            "required_aspect_ratio": required_aspect_ratio,
            "required_storyboard_panel_count_range": [2, 6],
            "semantics": "这一条 frame 是当前视频单元的一张多格分镜板图片，不是片段首帧或尾帧；storyboard_panels 描述分镜板内部画格。"
        }
    });
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

pub(in crate::video) fn build_first_frame_prompt_user(
    context: FramePromptContext<'_>,
    chapter_content: &str,
) -> Result<String, AppError> {
    let FramePromptContext {
        prompt_engine,
        project_id,
        chapter_summary,
        chapter_analysis,
        segment_plan,
        available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    } = context;
    let rules = video_builder_rules(prompt_engine, project_id)?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_first_frame_prompts",
        "task_instruction_chapter_video_first_frame_prompts",
    )?;
    let reference_slots = frame_reference_slots(reference_ids, None)?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_first_frame_prompts")?;
    let payload = json!({
        "task": task,
        "chapter": {
            "content": chapter_content,
            "summary": chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
            "analysis": chapter_analysis_context(chapter_analysis)
        },
        "previous_steps": {
            "chapter_video_segment_plan": segment_plan
        },
        "available_assets": available_assets,
        "constraints": {
            "style": style,
            "style_template_verbatim": style,
            "prompt_output_language": prompt_output_language.to_prompt_json(),
            "prompt_output_language_instruction": prompt_output_language.instruction(),
            "frame_prompt_required_sections": prompt_output_language.frame_sections(),
            "aspect_ratio": aspect_ratio,
            "reference_slots": reference_slots,
            "style_preservation_rule": rule_value(&rules, "single_frame_style_preservation_rule")?,
            "initial_frame_rule": rule_value(&rules, "initial_frame_rule")?,
            "frame_scope_rule": rule_value(&rules, "initial_frame_scope_rule")?,
            "frame_prompt_section_rule": rule_value(&rules, "frame_image_prompt_section_rule")?,
            "segment_directing_rule": rule_value(&rules, "initial_frame_segment_directing_rule")?,
            "material_coverage_rule": rule_value(&rules, "material_coverage_rule")?,
            "reference_slot_order_rule": frame_reference_slot_order_rule(),
            "prompt_density_rule": prompt_output_language.density_rule(),
            "image_input_semantics_rule": rule_value(&rules, "initial_frame_image_input_semantics_rule")?
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_first_frame_prompts",
            "required_root_field": "frames",
            "required_frame_fields": [
                "title",
                "scene_summary",
                "frame_prompt",
                "blocking_state",
                "lighting_state",
                "negative_prompt"
            ],
            "derived_frame_fields": [
                "frame_number",
                "duration_secs",
                "aspect_ratio",
                "segment_numbers"
            ],
            "frame_number_rule": "frames 数组只能包含 1 条记录；不要输出 frame_number、duration_secs、aspect_ratio、segment_numbers 或 reference_image_ids，后端会按第 1 个片段和用户选择补齐。"
        }
    });
    render_strict_json_user_prompt(prompt_engine, project_id, &user_instruction, &payload)
}

fn render_strict_json_user_prompt(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    instruction: &str,
    payload: &Value,
) -> Result<String, AppError> {
    let payload = pretty_json_with_root_key_order(payload, VIDEO_PAYLOAD_ROOT_KEY_ORDER)?;
    let mut variables = HashMap::new();
    variables.insert("instruction".to_string(), json!(instruction));
    variables.insert("payload".to_string(), json!(payload));
    prompt_engine.render(VIDEO_USER_REQUEST_TEMPLATE, &variables, Some(project_id))
}

fn chapter_analysis_context(chapter_analysis: Option<&Value>) -> Option<Value> {
    let analysis = chapter_analysis?;
    let mut object = serde_json::Map::new();
    if let Some(compressed) = analysis
        .get("summaries")
        .and_then(|v| v.get("compressed"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        object.insert(
            "compressed_summary".to_string(),
            Value::String(compressed.to_string()),
        );
    }
    if let Some(key_events) = analysis.get("key_events") {
        object.insert("key_events".to_string(), key_events.clone());
    }
    if let Some(metadata) = analysis.get("metadata") {
        object.insert("metadata".to_string(), metadata.clone());
    }
    if object.is_empty() {
        None
    } else {
        Some(Value::Object(object))
    }
}

#[cfg(test)]
mod tests;
