use super::asset_contract::optimized_prompt_asset_contract;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_orchestrator::video::PromptOutputLanguage;
use serde_json::{Value, json};

pub(super) fn video_prompt_optimization_source_artifact(
    prompt: &chapter_video_prompt::Model,
) -> Value {
    json!({
        "artifact_kind": "chapter_video_prompt",
        "current_prompt": {
            "prompt_id": prompt.id,
            "prompt_type": prompt.prompt_type.as_str(),
            "workflow_mode": prompt.workflow_mode.as_str(),
            "shot_number": prompt.shot_number,
            "title": prompt.title.as_deref(),
            "scene_summary": prompt.scene_summary.as_deref(),
            "duration_secs": prompt.duration_secs,
            "aspect_ratio": prompt.aspect_ratio.as_deref(),
            "visual_prompt": &prompt.visual_prompt,
            "motion_prompt": prompt.motion_prompt.as_deref(),
            "negative_prompt": prompt.negative_prompt.as_deref()
        }
    })
}

pub(super) fn build_video_prompt_optimization_dynamic_payload(
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Value {
    json!({
        "context_refs": {
            "current_prompt": "稳定上下文包 source_artifact.current_prompt"},
        "instruction": instruction,
        "prompt_output_language": prompt_output_language.to_prompt_json(),
        "prompt_output_language_instruction": prompt_output_language.instruction(),
        "output_contract": {
            "format": "json_object",
            "required_fields": ["visual_prompt", "motion_prompt", "negative_prompt"],
            "asset_contract": optimized_prompt_asset_contract(prompt, prompt_output_language),
            "field_rules": {
                "visual_prompt": "优化后的画面/主体提示词，不能为空；自然语言必须遵守 prompt_output_language_instruction；原文对白、引号内台词和角色实际说出口的短句必须保留原文写法，不要把中文对白翻成英文",
                "motion_prompt": "最终视频提示词必须返回完整 motion_prompt，不要写 null；帧提示词不使用 motion_prompt，可返回 null；自然语言必须遵守 prompt_output_language_instruction；原文对白、引号内台词和角色实际说出口的短句必须保留原文写法，不要把中文对白翻成英文",
                "negative_prompt": "优化后的负面提示词；如果不需要修改，返回 null 表示保留原值；自然语言必须遵守 prompt_output_language_instruction"
            },
            "forbidden": ["markdown", "code_fence", "extra_fields", "json_inside_prompt_text", "internal_image_id", "upload_status_text", "motion_timeline_duplicate"]
        }
    })
}
