use super::super::prompt_is_first_frame_chain;
use crate::actions::video_generation::{
    CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME, CHAPTER_VIDEO_PROVIDER_PROMPT_NAME,
    MIN_PROVIDER_MOTION_PROMPT_CHARS, MIN_PROVIDER_VISUAL_PROMPT_CHARS, ProviderPromptImageBinding,
    validate_provider_model_text,
};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_orchestrator::video::PromptOutputLanguage;
use serde_json::{Value, json};
use std::collections::HashMap;

pub(super) struct ProviderPromptPlan {
    pub(super) prompt_name: &'static str,
    pub(super) variables: HashMap<String, Value>,
}

pub(super) fn build_provider_prompt_plan(
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
    allow_preview_placeholders: bool,
) -> Result<ProviderPromptPlan, String> {
    let prompt_name = if prompt_is_first_frame_chain(prompt)? {
        validate_first_frame_chain_metadata(prompt)?;
        CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME
    } else {
        validate_standard_metadata(prompt)?;
        CHAPTER_VIDEO_PROVIDER_PROMPT_NAME
    };
    let variables =
        build_common_provider_prompt_variables(prompt, image_binding, allow_preview_placeholders)?;
    Ok(ProviderPromptPlan {
        prompt_name,
        variables,
    })
}

pub(in crate::actions::video_generation) fn provider_specific_negative_prompt(
    specific: Option<&str>,
) -> Option<String> {
    specific
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !matches!(*value, "无" | "无特殊负面词" | "无特殊要求"))
        .map(str::to_string)
}

fn validate_standard_metadata(prompt: &chapter_video_prompt::Model) -> Result<(), String> {
    if prompt.metadata_json.is_none() {
        return Err(format!(
            "视频提示词 #{} 缺少结构化元数据，请重新生成视频提示词后再生成视频",
            prompt.id
        ));
    }
    Ok(())
}

fn validate_first_frame_chain_metadata(prompt: &chapter_video_prompt::Model) -> Result<(), String> {
    let metadata = prompt.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少结构化元数据，请重新生成首帧串联视频提示词后再生成视频",
            prompt.id
        )
    })?;
    if prompt.shot_number <= 1 {
        required_object(metadata, "start_first_frame", prompt.id)?;
    }
    Ok(())
}

fn build_common_provider_prompt_variables(
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
    allow_preview_placeholders: bool,
) -> Result<HashMap<String, Value>, String> {
    let prompt_output_language = PromptOutputLanguage::from_metadata(prompt.metadata_json.as_ref());
    let visual_prompt = validate_provider_model_text(
        &prompt.visual_prompt,
        "visual_prompt",
        prompt.id,
        prompt_output_language,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        allow_preview_placeholders,
    )?;
    let raw_motion_prompt = prompt
        .motion_prompt
        .as_deref()
        .ok_or_else(|| format!("视频提示词 #{} 缺少 motion_prompt", prompt.id))?;
    let motion_prompt = validate_provider_model_text(
        raw_motion_prompt,
        "motion_prompt",
        prompt.id,
        prompt_output_language,
        MIN_PROVIDER_MOTION_PROMPT_CHARS,
        allow_preview_placeholders,
    )?;
    let _aspect_ratio = required_prompt_aspect_ratio(prompt)?;
    let mut variables = HashMap::new();
    variables.insert(
        "has_first_frame_image".to_string(),
        json!(image_binding.has_first_frame_image),
    );
    variables.insert("visual_prompt".to_string(), json!(visual_prompt));
    variables.insert("motion_prompt".to_string(), json!(motion_prompt));
    variables.insert(
        "subject_asset_bindings".to_string(),
        json!(provider_subject_asset_bindings(image_binding)),
    );
    Ok(variables)
}

fn provider_subject_asset_bindings(image_binding: ProviderPromptImageBinding) -> Option<String> {
    let mut lines = Vec::new();
    if image_binding.has_first_frame_image {
        lines.push(
            "- 首帧输入图片是视频第 0 秒画面；从该画面的构图、人物站位、姿态、道具和光影自然推进。"
                .to_string(),
        );
    }
    if image_binding.has_last_frame_image {
        lines.push(
            "- 尾帧输入图片是视频结尾目标；收束到该画面的构图、人物站位、姿态、道具和光影。"
                .to_string(),
        );
    }
    if image_binding.reference_image_count > 0 {
        if image_binding.has_storyboard_reference_image {
            lines.push(format!(
                "- 本请求按顺序输入 {} 张参考图；第 1 张参考图是当前片段分镜图，只用于约束构图、主体姿态、空间关系和光影，不是视频首帧定格；其余参考图继续按输入顺序用于人物、场景、道具、服装和风格一致性。",
                image_binding.reference_image_count
            ));
        } else {
            lines.push(format!(
                "- 本请求按顺序输入 {} 张参考图；提示词中的“参考图 N”对应第 N 张输入图片，只用于人物、场景、道具、服装、边界状态和风格一致性。",
                image_binding.reference_image_count
            ));
        }
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn required_prompt_aspect_ratio(prompt: &chapter_video_prompt::Model) -> Result<&str, String> {
    let value = prompt
        .aspect_ratio
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("视频提示词 #{} 缺少 aspect_ratio", prompt.id))?;
    Ok(value)
}

fn required_object<'a>(
    metadata: &'a Value,
    key: &str,
    prompt_id: i64,
) -> Result<&'a Value, String> {
    let value = metadata.get(key).ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少 {} 元数据，请重新生成视频提示词后再生成视频",
            prompt_id, key
        )
    })?;
    if !value.is_object() {
        return Err(format!("视频提示词 #{} 的 {} 元数据无效", prompt_id, key));
    }
    Ok(value)
}
