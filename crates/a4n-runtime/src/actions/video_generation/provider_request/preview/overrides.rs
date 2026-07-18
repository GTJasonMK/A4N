use super::super::*;
use super::notes::append_provider_input_note;

pub(in crate::actions::video_generation) fn apply_chapter_video_prompt_preview_override(
    prompt: &mut chapter_video_prompt::Model,
    input: Option<PreviewChapterVideoPromptOverrideInput>,
    note: &mut Option<String>,
) {
    let Some(input) = input else {
        return;
    };
    append_provider_input_note(
        note,
        "预览使用当前编辑区草稿覆盖已保存提示词；真实生成仍会读取已保存提示词，请保存后再生成。",
    );
    if let Some(visual_prompt) = input.visual_prompt {
        prompt.visual_prompt = visual_prompt.trim().to_string();
    }
    if let Some(motion_prompt) = input.motion_prompt {
        prompt.motion_prompt = motion_prompt
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
    }
    if let Some(negative_prompt) = input.negative_prompt {
        prompt.negative_prompt = negative_prompt
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
    }
    if let Some(duration_secs) = input.duration_secs {
        if duration_secs > 0 {
            prompt.duration_secs = duration_secs;
        } else {
            append_provider_input_note(
                note,
                format!(
                    "草稿 duration_secs={} 无效；预览使用 1 秒占位，真实生成仍需保存有效时长。",
                    duration_secs
                ),
            );
            prompt.duration_secs = 1;
        }
    }
    if let Some(reference_image_ids_json) = input.reference_image_ids_json {
        prompt.reference_image_ids_json = reference_image_ids_json;
    }
}
