use super::super::super::super::*;
use super::super::super::notes::{
    append_optional_input_note, append_video_request_prompt_length_note,
};

pub(super) fn gemini_preview_payload(
    context: &ChapterVideoProviderRequestContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    let request = GeminiVeoVideoProvider::request_body_preview(&context.request)
        .map_err(|e| e.to_string())?;
    let mut note = format!(
        "Gemini Veo 原生 `predictLongRunning` 请求；首尾帧媒体输入：{} / {}；provider 参考图：{}；提示词参考图：{}；预览用 inlineData 占位展示实际请求形状，生成时发送真实图片 data URL。",
        context.first_frame_image_url_available,
        context.last_frame_image_url_available,
        context.provider_reference_image_count,
        context.prompt_reference_image_count
    );
    append_optional_input_note(&mut note, context.provider_input_note.as_deref());
    append_video_request_prompt_length_note(&mut note, &context.request);
    return Ok((
        "video_generation_gemini_veo",
        "Gemini Veo predictLongRunning 请求",
        note,
        request,
    ));
}
