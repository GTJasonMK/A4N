use crate::actions::video_prompt_text::{
    MIN_FRAME_PROMPT_CHARS, MIN_PROVIDER_MOTION_PROMPT_CHARS, MIN_PROVIDER_VISUAL_PROMPT_CHARS,
};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_orchestrator::video::PromptOutputLanguage;
use serde_json::{Value, json};

pub(super) fn optimized_prompt_asset_contract(
    prompt: &chapter_video_prompt::Model,
    prompt_output_language: PromptOutputLanguage,
) -> Value {
    match prompt.prompt_type.as_str() {
        "video_prompt" => json!({
            "kind": "video_prompt",
            "visual_prompt_required_sections": prompt_output_language.visual_sections(),
            "motion_prompt_required_sections": prompt_output_language.motion_sections(),
            "motion_prompt_required": true,
            "motion_prompt_must_not_repeat_timing_breakdown": true,
            "minimum_lengths": {
                "visual_prompt_chars": MIN_PROVIDER_VISUAL_PROMPT_CHARS,
                "motion_prompt_chars": MIN_PROVIDER_MOTION_PROMPT_CHARS
            },
            "forbidden_terms": {
                "upload_status_examples": ["无输入首帧", "没有输入尾帧"],
                "internal_workflow_examples": ["首帧串联", "provider", "image 字段", "generated_image #"]
            }
        }),
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" => json!({
            "kind": "frame_prompt",
            "visual_prompt_is_frame_prompt": true,
            "visual_prompt_required_sections": prompt_output_language.frame_sections(),
            "motion_prompt_required": false,
            "minimum_lengths": {
                "visual_prompt_chars": MIN_FRAME_PROMPT_CHARS
            },
            "forbidden_terms": {
                "internal_workflow_examples": ["首帧串联", "provider", "image 字段", "generated_image #"]
            }
        }),
        _ => json!({
            "kind": "text_prompt",
            "motion_prompt_required": false
        }),
    }
}
