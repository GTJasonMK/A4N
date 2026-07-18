use super::super::super::super::*;
use super::super::super::notes::{
    append_optional_input_note, append_video_request_prompt_length_note,
};

pub(super) fn openai_compatible_preview_payload(
    context: &ChapterVideoProviderRequestContext,
) -> (&'static str, &'static str, String, Value) {
    let mut note = format!(
        "OpenAI 兼容视频 `/videos/generations` 请求；provider 参考图：{}；提示词参考图：{}；首尾帧媒体输入：{} / {}；预览省略真实媒体输入，生成时默认发送图片 data URL；如设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=public_url，则发送公网 URL；如设置为 remote_temp_upload，则先上传到远端临时媒体接口再发送返回 URL。",
        context.provider_reference_image_count,
        context.prompt_reference_image_count,
        context.first_frame_image_url_available,
        context.last_frame_image_url_available
    );
    append_optional_input_note(&mut note, context.provider_input_note.as_deref());
    append_video_request_prompt_length_note(&mut note, &context.request);
    (
        "video_generation_openai_compatible",
        "OpenAI 兼容 /videos/generations 请求",
        note,
        HttpVideoProvider::request_body_preview(&context.request),
    )
}
